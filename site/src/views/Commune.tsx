import { useEffect, useRef, useState } from 'react'
import { useI18n } from '../i18n'
import { Perception } from '../lib/perception'
import { acquireSenses, releaseStream, type MediaResult } from '../lib/media'
import { idleSenses } from '../lib/idle'
import './commune.css'

// ── Body bridge ───────────────────────────────────────────────────────────────
// Connects to aborn-gateway via WebSocket. On each Perception.read() tick,
// packs the Senses struct into a compact binary frame (0x01) and sends it.
// Receives vocal PCM (0x02) and plays it; receives surface pixels (0x04)
// and draws them to the surface canvas; receives life-state pulses (0x03)
// and surfaces them to the UI.

const GATEWAY_WS = (() => {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${proto}//${location.host}/ws`
})()

const TAG_VOCAL       = 0x02
const TAG_STATE       = 0x03
const TAG_SURFACE     = 0x04
const TAG_SURFACE_GZ  = 0x05   // gzip-compressed surface frame (body→gateway→browser)

// Inflate a gzip-compressed payload using the platform DecompressionStream.
// Supported on iOS Safari 16.4+, Chrome/Edge 80+, Firefox 113+ — i.e. all
// current mobile browsers. Returns the raw bytes (a 0x04 surface frame).
async function gunzip(payload: Uint8Array): Promise<Uint8Array> {
  const ds = new DecompressionStream('gzip')
  // Copy into a fresh ArrayBuffer-backed view so the Blob part type is exact
  // (the incoming view may be backed by a SharedArrayBuffer-typed buffer).
  const part = payload.slice()
  const stream = new Blob([part]).stream().pipeThrough(ds)
  return new Uint8Array(await new Response(stream).arrayBuffer())
}
const BANDS       = 64
const GW          = 160   // camera grid width (source); height is CH*DS = 120
// The browser used to send a 230 KB float32 frame (160×120×3 f32) — at 10 fps it
// saturated the Cloudflare tunnel and was dropped (body blind → black screen). It
// now sends the COMPACT frame below; the gateway expands it back to 160×120 f32
// for the (local) body, so the body + nursery feeder are unchanged.
// Compact uplink: 40×30 u8 grid (downsampled 4×, box-averaged) + same 76 f32
// scalars. tag(1) + 304 + 40×30×3 = 3905 bytes (≈59× smaller, ~39 KB/s @ 10 fps).
const TAG_SENSES_COMPACT = 0x06
const CW          = 40
const CH          = 30
const DS          = 4   // 160/40 = 120/30
const COMPACT_FRAME_BYTES = 1 + (2 + BANDS + 2 + 8) * 4 + CW * CH * 3  // 3905

export interface LifeState {
  reserve: number  // 0..1 — energy/vitality level
  vstate: number   // 0=Alive, 1=Standby
}

type DisconnectReason = 'capacity' | 'body-failed' | 'session-limit' | 'kicked'

function mapCloseCode(code: number): DisconnectReason {
  if (code === 1013) return 'capacity'
  if (code === 1011) return 'body-failed'
  if (code === 1001) return 'session-limit'
  return 'kicked'
}

// 4× box-average downsample of a GW×GH float (0..1) plane → CW×CH u8 (0..255),
// written planar into `out` at `outOff`. Averaging (not point-sampling) keeps the
// shrunk grid representative instead of aliased.
function downsampleU8(src: ArrayLike<number>, out: Uint8Array, outOff: number): void {
  for (let dy = 0; dy < CH; dy++) {
    for (let dx = 0; dx < CW; dx++) {
      let sum = 0
      for (let oy = 0; oy < DS; oy++) {
        const row = (dy * DS + oy) * GW + dx * DS
        for (let ox = 0; ox < DS; ox++) sum += src[row + ox] ?? 0
      }
      const v = Math.round((sum / (DS * DS)) * 255)
      out[outOff + dy * CW + dx] = v < 0 ? 0 : v > 255 ? 255 : v
    }
  }
}

function packSenses(s: import('../lib/perception').Senses): ArrayBuffer {
  const buf  = new ArrayBuffer(COMPACT_FRAME_BYTES)
  const view = new DataView(buf)
  let off = 0
  view.setUint8(off++, TAG_SENSES_COMPACT)
  view.setFloat32(off, s.audioLevel, true); off += 4
  view.setFloat32(off, s.audioFlux,  true); off += 4
  for (let b = 0; b < BANDS; b++) {
    view.setFloat32(off, s.bands[b] ?? 0, true); off += 4
  }
  view.setFloat32(off, s.motion, true); off += 4
  view.setFloat32(off, s.luma,   true); off += 4
  // 8 aux features (full f32 precision — small and important)
  view.setFloat32(off, s.spectralCentroid ?? 0, true); off += 4
  view.setFloat32(off, s.spectralFlatness  ?? 0, true); off += 4
  view.setFloat32(off, s.spectralRolloff   ?? 0, true); off += 4
  view.setFloat32(off, s.spectralSpread    ?? 0, true); off += 4
  view.setFloat32(off, s.zcr               ?? 0, true); off += 4
  view.setFloat32(off, s.hnr               ?? 0, true); off += 4
  view.setFloat32(off, s.onsetStrength     ?? 0, true); off += 4
  view.setFloat32(off, s.temporalFlux      ?? 0, true); off += 4
  // RGB grid downsampled to CW×CH u8, planar R,G,B. Shares `buf` with the
  // DataView above (the f32 scalars occupy [1,305), the u8 grid [305,3905)).
  const u8 = new Uint8Array(buf)
  const gridR = s.gridR ?? s.grid  // fallback to luma if RGB not available
  const gridG = s.gridG ?? s.grid
  const gridB = s.gridB ?? s.grid
  downsampleU8(gridR, u8, off); off += CW * CH
  downsampleU8(gridG, u8, off); off += CW * CH
  downsampleU8(gridB, u8, off); off += CW * CH
  return buf
}

function xorApply(data: Uint8Array, key: Uint8Array): Uint8Array {
  const out = new Uint8Array(data.length)
  for (let i = 0; i < data.length; i++) out[i] = data[i] ^ key[i % 4]
  return out
}

function useBodyBridge(
  running: boolean,
  getSenses: () => import('../lib/perception').Senses,
  surfaceRef: React.RefObject<HTMLCanvasElement | null>,
  // Pre-created AudioContext from begin() — created inside the user-gesture
  // promise chain so iOS Safari allows it in running state immediately.
  audioCtxRef: React.RefObject<AudioContext | null>,
  onLifeState: (s: LifeState) => void,
  onDisconnect: (reason: DisconnectReason) => void,
) {
  const wsRef       = useRef<WebSocket | null>(null)
  const nextTimeRef = useRef(0)
  const xorKeyRef   = useRef<Uint8Array | null>(null)
  // True while ws is intentionally open; set false before cleanup close.
  const liveRef     = useRef(false)

  useEffect(() => {
    if (!running) return
    const actx = audioCtxRef.current
    if (!actx) return   // begin() must have created this already

    // Belt-and-suspenders: attempt resume on every interaction. The AudioContext
    // was created in begin() (gesture context) so it should already be running;
    // this handles rare races on some iOS builds.
    const unlockAudio = () => { void actx.resume() }
    document.addEventListener('touchstart', unlockAudio, { passive: true })
    document.addEventListener('click',      unlockAudio, { passive: true })

    // Reconnect bookkeeping (see connect()/onclose below).
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null
    let attempts = 0
    const MAX_ATTEMPTS = 6

    const handleMessage = (ev: MessageEvent) => {
      if (!(ev.data instanceof ArrayBuffer)) return
      const raw = new Uint8Array(ev.data)
      const key = xorKeyRef.current

      if (!key) {
        if (raw.length === 5 && raw[0] === 0x00) xorKeyRef.current = raw.slice(1, 5)
        return
      }

      const buf  = xorApply(raw, key)
      const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength)
      const tag  = view.getUint8(0)

      if (tag === TAG_VOCAL) {
        const n    = view.getUint16(1, true)
        if (n === 0) return  // guard: empty frame from Idle vocal organ
        const RATE = 44100   // match VOCAL_SAMPLE_RATE in net_adapter.rs
        const abuf = actx.createBuffer(1, n, RATE)
        const chan  = abuf.getChannelData(0)
        for (let i = 0; i < n; i++) chan[i] = view.getInt16(3 + i * 2, true) / 32768.0
        const src = actx.createBufferSource()
        src.buffer = abuf
        src.connect(actx.destination)
        const now  = actx.currentTime
        const when = Math.max(now, nextTimeRef.current)
        src.start(when)
        nextTimeRef.current = when + abuf.duration

      } else if (tag === TAG_STATE) {
        // [0x03][reserve:f32le][vital_state:u8][self_coherence:f32le][...]
        if (buf.byteLength >= 6) {
          const reserve = Math.min(1, Math.max(0, view.getFloat32(1, true)))
          const vstate  = view.getUint8(5)
          onLifeState({ reserve, vstate })
        }

      } else if (tag === TAG_SURFACE) {
        drawSurface(buf)
      } else if (tag === TAG_SURFACE_GZ) {
        // Decompress off the hot path; surface frames are independent, so a
        // late/out-of-order inflate just paints a slightly older frame — fine.
        void gunzip(buf.subarray(1)).then(drawSurface).catch(() => {})
      }
    }

    function drawSurface(frame: Uint8Array) {
      if (frame.byteLength < 5) return
      const dv = new DataView(frame.buffer, frame.byteOffset, frame.byteLength)
      const w = dv.getUint16(1, true)
      const h = dv.getUint16(3, true)
      if (frame.byteLength < 5 + w * h * 4) return
      const cv = surfaceRef.current
      if (!cv) return
      const ctx2d = cv.getContext('2d')
      if (!ctx2d) return
      if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h }
      const src = new Uint8Array(frame.buffer, frame.byteOffset + 5, w * h * 4)
      const img = ctx2d.createImageData(w, h)
      const GAIN = 3
      for (let i = 0; i < src.length; i += 4) {
        img.data[i]   = Math.min(255, src[i]   * GAIN)
        img.data[i+1] = Math.min(255, src[i+1] * GAIN)
        img.data[i+2] = Math.min(255, src[i+2] * GAIN)
        img.data[i+3] = 255
      }
      ctx2d.putImageData(img, 0, 0)
    }

    const connect = () => {
      const ws = new WebSocket(GATEWAY_WS)
      ws.binaryType = 'arraybuffer'
      wsRef.current  = ws
      xorKeyRef.current = null
      nextTimeRef.current = 0
      void actx.resume()
      ws.onopen = () => { attempts = 0 }   // a successful open resets the backoff
      ws.onmessage = handleMessage
      ws.onclose = (ev) => {
        if (!liveRef.current) return  // intentional cleanup close — no error
        // Transient drop (1006 abnormal = the tunnel killed an idle/congested
        // socket; 1011 body-failed; 1012 restart) → reconnect with exponential
        // backoff instead of dumping the user to the disconnect modal. Deliberate
        // closes (1013 capacity, 1001 session-limit) and exhausted retries are
        // terminal.
        const code = ev.code
        const transient = code === 1006 || code === 1011 || code === 1012
        if (transient && attempts < MAX_ATTEMPTS) {
          attempts++
          const delay = Math.min(8000, 500 * 2 ** (attempts - 1))  // 0.5→8 s
          reconnectTimer = setTimeout(connect, delay)
          return
        }
        onDisconnect(mapCloseCode(code))
      }
    }

    liveRef.current = true
    connect()

    // Senses upload: the compact frame is ~3.9 KB (40×30 u8 grid + 76 scalars),
    // ~39 KB/s @ 100 ms — within the Cloudflare tunnel's ~100 KB/s headroom (the
    // old 230 KB float frame saturated it and was dropped → black screen). Send
    // only when the socket isn't already backed up, on whichever socket is
    // currently live (survives reconnects via wsRef).
    const interval = setInterval(() => {
      const ws = wsRef.current
      if (!ws || ws.readyState !== WebSocket.OPEN) return
      const key = xorKeyRef.current
      if (!key) return
      if (ws.bufferedAmount > 64 * 1024) return  // link saturated — skip this frame
      const plain = new Uint8Array(packSenses(getSenses()))
      ws.send(xorApply(plain, key))
    }, 100)

    return () => {
      liveRef.current = false  // mark intentional before close
      if (reconnectTimer) clearTimeout(reconnectTimer)
      clearInterval(interval)
      document.removeEventListener('touchstart', unlockAudio)
      document.removeEventListener('click',      unlockAudio)
      wsRef.current?.close()
      wsRef.current = null
      xorKeyRef.current = null
      // AudioContext lifecycle owned by Commune — do NOT close here.
    }
  }, [running])
}

type Phase =
  | { k: 'requesting' }
  | { k: 'live' }
  | { k: 'denied' }
  | { k: 'unsupported' }
  | { k: 'disconnected'; reason: DisconnectReason }

function useCallTimer(running: boolean) {
  const [secs, setSecs] = useState(0)
  useEffect(() => {
    if (!running) { setSecs(0); return }
    const id = setInterval(() => setSecs(s => s + 1), 1000)
    return () => clearInterval(id)
  }, [running])
  const m = String(Math.floor(secs / 60)).padStart(2, '0')
  const s = String(secs % 60).padStart(2, '0')
  return { display: `${m}:${s}`, secs }
}

export function Commune({ onLeave }: { onLeave: (secs: number) => void }) {
  const { t } = useI18n()
  const [phase, setPhase] = useState<Phase>({ k: 'requesting' })
  const [muted, setMuted] = useState(false)
  const [camOff, setCamOff] = useState(false)
  const [hasVideo, setHasVideo] = useState(true)
  const [lifeState, setLifeState] = useState<LifeState | null>(null)

  const perceptionRef  = useRef<Perception | null>(null)
  const streamRef      = useRef<MediaStream | null>(null)
  const selfVideoRef   = useRef<HTMLVideoElement>(null)
  const surfaceRef     = useRef<HTMLCanvasElement>(null)
  const fallback       = useRef(idleSenses())
  // AudioContext owned here so it can be created inside the user-gesture
  // promise chain in begin(), before React state updates consume the gesture.
  const audioCtxRef    = useRef<AudioContext | null>(null)

  const timer = useCallTimer(phase.k === 'live')

  const handleDisconnect = (reason: DisconnectReason) => {
    setPhase({ k: 'disconnected', reason })
  }

  const begin = async () => {
    setPhase({ k: 'requesting' })
    setLifeState(null)
    // Close any leftover AudioContext from a previous session.
    if (audioCtxRef.current) {
      audioCtxRef.current.close().catch(() => {})
      audioCtxRef.current = null
    }
    const res: MediaResult = await acquireSenses()
    if (!res.ok) {
      setPhase({ k: res.reason === 'unsupported' ? 'unsupported' : 'denied' })
      return
    }
    streamRef.current = res.stream
    setHasVideo(res.video)
    setCamOff(!res.video)
    if (res.video && selfVideoRef.current) {
      selfVideoRef.current.srcObject = res.stream
      selfVideoRef.current.muted = true
      selfVideoRef.current.playsInline = true
      void selfVideoRef.current.play().catch(() => {})
    }
    const perc = new Perception()
    await perc.attach(res.stream)
    perc.resumeAudio()
    perceptionRef.current = perc
    // Create AudioContext HERE — still inside the promise chain of the user
    // gesture (media permission grant), so iOS Safari starts it in running
    // state and resume() is guaranteed to succeed.
    const Ctor = (window.AudioContext || (window as any).webkitAudioContext) as typeof AudioContext
    const actx = new Ctor()
    void actx.resume()
    audioCtxRef.current = actx
    setPhase({ k: 'live' })
  }

  useEffect(() => {
    void begin()
    return () => {
      perceptionRef.current?.dispose()
      releaseStream(streamRef.current)
      perceptionRef.current = null
      streamRef.current = null
      audioCtxRef.current?.close().catch(() => {})
      audioCtxRef.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const leave = () => {
    const secs = timer.secs
    perceptionRef.current?.dispose()
    releaseStream(streamRef.current)
    perceptionRef.current = null
    streamRef.current = null
    audioCtxRef.current?.close().catch(() => {})
    audioCtxRef.current = null
    onLeave(secs)
  }

  const retry = () => {
    releaseStream(streamRef.current)
    streamRef.current = null
    void begin()
  }

  const toggleMute = () => {
    const stream = streamRef.current
    if (!stream) return
    const newMuted = !muted
    stream.getAudioTracks().forEach(t => { t.enabled = !newMuted })
    setMuted(newMuted)
  }

  const toggleCam = () => {
    const stream = streamRef.current
    if (!stream) return
    const newOff = !camOff
    stream.getVideoTracks().forEach(t => { t.enabled = !newOff })
    setCamOff(newOff)
  }

  const read = () => perceptionRef.current ? perceptionRef.current.read() : fallback.current

  useBodyBridge(phase.k === 'live', read, surfaceRef, audioCtxRef, setLifeState, handleDisconnect)

  const isLive = phase.k === 'live'
  const isStandby = lifeState?.vstate === 1
  const reserve = lifeState?.reserve ?? 1

  return (
    <div className="commune">
      {/* Surface canvas — raw body visual from surface_render.rs */}
      <canvas ref={surfaceRef} className="commune__surface" aria-hidden="true" />

      {/* No-vision hint: when the camera is off or was never granted, the surface
          is legitimately black (aborn renders what it SEES). Explain it instead
          of leaving a dead black screen, and offer one tap to enable the camera. */}
      {isLive && (camOff || !hasVideo) && (
        <div className="commune__novision fade-in">
          <CamOffIcon />
          <p className="commune__novision-text">{t('commune.novision')}</p>
          <button
            className="commune__novision-btn"
            onClick={() => (hasVideo ? toggleCam() : retry())}
          >
            {t('commune.enable-cam')}
          </button>
        </div>
      )}

      {/* Top-left: live indicator with life-state feedback */}
      {isLive && (
        <div className="commune__remote-label fade-in">
          <span
            className={`commune__remote-dot${isStandby ? ' commune__remote-dot--standby' : ''}`}
            style={{ '--reserve': reserve } as React.CSSProperties}
          />
          <span className="commune__remote-name">aborn</span>
          {isStandby && (
            <span className="commune__standby-badge">{t('commune.standby')}</span>
          )}
          <span className="commune__timer">{timer.display}</span>
        </div>
      )}

      {/* Reserve bar — subtle energy indicator below the pill */}
      {isLive && lifeState && (
        <div className="commune__reserve-bar" aria-hidden="true">
          <div
            className={`commune__reserve-fill${isStandby ? ' commune__reserve-fill--standby' : ''}`}
            style={{ width: `${reserve * 100}%` }}
          />
        </div>
      )}

      {/* Self-view PiP */}
      <div className={`commune__self ${isLive ? 'commune__self--visible' : ''} ${camOff ? 'commune__self--off' : ''}`}>
        <video ref={selfVideoRef} className="commune__self-video" muted playsInline autoPlay />
        {camOff && <div className="commune__self-off-icon"><CamOffIcon /></div>}
      </div>

      {/* Perception hint */}
      {isLive && (
        <p className="commune__hint fade-in">{t('commune.hint')}</p>
      )}

      {/* Control bar */}
      {isLive && (
        <div className="commune__controls fade-in">
          <ControlBtn active={muted} onClick={toggleMute} label={muted ? t('commune.unmute') : t('commune.mute')}>
            {muted ? <MicOffIcon /> : <MicIcon />}
          </ControlBtn>
          <button className="commune__hangup" onClick={leave} aria-label={t('commune.leave')}>
            <HangupIcon />
          </button>
          <ControlBtn active={camOff} onClick={toggleCam} label={camOff ? t('commune.camon') : t('commune.camoff')}>
            {camOff ? <CamOffIcon /> : <CamIcon />}
          </ControlBtn>
        </div>
      )}

      {phase.k === 'requesting' && (
        <div className="commune__modal fade-in">
          <span className="commune__spinner" />
          <p>{t('commune.requesting')}</p>
        </div>
      )}

      {(phase.k === 'denied' || phase.k === 'unsupported') && (
        <div className="commune__modal fade-in">
          <h2>{t(phase.k === 'denied' ? 'commune.denied.title' : 'commune.unsupported.title')}</h2>
          <p>{t(phase.k === 'denied' ? 'commune.denied.body' : 'commune.unsupported.body')}</p>
          <div className="commune__actions">
            {phase.k === 'denied' && (
              <button className="commune__btn commune__btn--primary" onClick={retry}>
                {t('commune.retry')}
              </button>
            )}
            <button className="commune__btn" onClick={() => onLeave(0)}>{t('commune.leave')}</button>
          </div>
        </div>
      )}

      {phase.k === 'disconnected' && (
        <div className="commune__modal fade-in">
          <h2>{t(
            phase.reason === 'capacity'
              ? 'commune.capacity.title'
              : phase.reason === 'body-failed'
                ? 'commune.body-failed.title'
                : 'commune.disconnected.title'
          )}</h2>
          <p>{t(
            phase.reason === 'capacity'
              ? 'commune.capacity.body'
              : phase.reason === 'body-failed'
                ? 'commune.body-failed.body'
                : 'commune.disconnected.body'
          )}</p>
          <div className="commune__actions">
            {phase.reason !== 'kicked' && (
              <button className="commune__btn commune__btn--primary" onClick={retry}>
                {t('commune.retry')}
              </button>
            )}
            <button className="commune__btn" onClick={() => onLeave(0)}>{t('commune.leave')}</button>
          </div>
        </div>
      )}
    </div>
  )
}

function ControlBtn({ active, onClick, label, children }: {
  active?: boolean; onClick: () => void; label: string; children: React.ReactNode
}) {
  return (
    <button className={`commune__ctrl${active ? ' commune__ctrl--active' : ''}`}
      onClick={onClick} aria-label={label} title={label}>
      {children}
    </button>
  )
}

function MicIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round">
      <rect x="9" y="2" width="6" height="12" rx="3" />
      <path d="M5 10a7 7 0 0 0 14 0" />
      <line x1="12" y1="19" x2="12" y2="22" />
      <line x1="9" y1="22" x2="15" y2="22" />
    </svg>
  )
}
function MicOffIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round">
      <line x1="2" y1="2" x2="22" y2="22" />
      <path d="M18.89 13.23A7 7 0 0 0 19 10" />
      <path d="M5 10a7 7 0 0 0 11.08 5.76" />
      <path d="M9 9v3a3 3 0 0 0 5.12 2.12" />
      <rect x="9" y="2" width="6" height="8" rx="3" />
      <line x1="12" y1="19" x2="12" y2="22" />
      <line x1="9" y1="22" x2="15" y2="22" />
    </svg>
  )
}
function CamIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round">
      <path d="M23 7l-7 5 7 5V7z" />
      <rect x="1" y="5" width="15" height="14" rx="2" />
    </svg>
  )
}
function CamOffIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round">
      <line x1="2" y1="2" x2="22" y2="22" />
      <path d="M10.66 6H14a2 2 0 0 1 2 2v2.34l1 1L23 7v10" />
      <path d="M16 16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h2l10 10z" />
    </svg>
  )
}
function HangupIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor">
      <path d="M6.6 10.8c1.4 2.8 3.8 5.1 6.6 6.6l2.2-2.2c.3-.3.7-.4 1-.2 1.1.4 2.3.6 3.6.6.6 0 1 .4 1 1V20c0 .6-.4 1-1 1-9.4 0-17-7.6-17-17 0-.6.4-1 1-1h3.5c.6 0 1 .4 1 1 0 1.3.2 2.5.6 3.6.1.3 0 .7-.2 1L6.6 10.8z"/>
    </svg>
  )
}
