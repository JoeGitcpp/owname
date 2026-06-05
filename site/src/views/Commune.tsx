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
// and draws them to the surface canvas.

const GATEWAY_WS = (() => {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${proto}//${location.host}/ws`
})()

const TAG_SENSES = 0x01
const TAG_VOCAL  = 0x02
const TAG_SURFACE = 0x04
const BANDS      = 12
const GW         = 32
const GH         = 24
const FRAME_BYTES = 1 + (2 + BANDS) * 4 + GW * GH * 4

function packSenses(s: import('../lib/perception').Senses): ArrayBuffer {
  const buf  = new ArrayBuffer(FRAME_BYTES)
  const view = new DataView(buf)
  let off    = 0
  view.setUint8(off++, TAG_SENSES)
  view.setFloat32(off, s.audioLevel,   true); off += 4
  view.setFloat32(off, s.audioFlux,    true); off += 4
  for (let b = 0; b < BANDS; b++) {
    view.setFloat32(off, s.bands[b] ?? 0, true); off += 4
  }
  view.setFloat32(off, s.motion, true); off += 4
  view.setFloat32(off, s.luma,   true); off += 4
  for (let gy = 0; gy < GH; gy++) {
    for (let gx = 0; gx < GW; gx++) {
      const sx = Math.floor(gx * s.gw / GW)
      const sy = Math.floor(gy * s.gh / GH)
      view.setFloat32(off, s.grid[sy * s.gw + sx] ?? 0, true); off += 4
    }
  }
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
) {
  const wsRef        = useRef<WebSocket | null>(null)
  const audioCtxRef  = useRef<AudioContext | null>(null)
  const nextTimeRef  = useRef(0)
  const xorKeyRef    = useRef<Uint8Array | null>(null)

  useEffect(() => {
    if (!running) return
    const ws = new WebSocket(GATEWAY_WS)
    ws.binaryType = 'arraybuffer'
    wsRef.current  = ws
    xorKeyRef.current = null

    const Ctor = (window.AudioContext || (window as any).webkitAudioContext) as typeof AudioContext
    const actx = new Ctor()
    audioCtxRef.current = actx
    nextTimeRef.current = 0

    // iOS Safari creates AudioContext in suspended state — resume immediately.
    // The permission tap that brought us to 'live' phase is a valid gesture context.
    actx.resume().catch(() => {})

    // Belt-and-suspenders: also resume on first user interaction inside Commune
    // in case the above attempt is still too late in the gesture chain.
    const unlockAudio = () => {
      audioCtxRef.current?.resume().catch(() => {})
      document.removeEventListener('touchstart', unlockAudio, true)
      document.removeEventListener('click',      unlockAudio, true)
    }
    document.addEventListener('touchstart', unlockAudio, { once: true, capture: true })
    document.addEventListener('click',      unlockAudio, { once: true, capture: true })

    ws.onmessage = (ev: MessageEvent) => {
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
        const RATE = 22050
        const abuf = actx.createBuffer(1, n, RATE)
        const chan  = abuf.getChannelData(0)
        for (let i = 0; i < n; i++) chan[i] = view.getFloat32(3 + i * 4, true)
        const src = actx.createBufferSource()
        src.buffer = abuf
        src.connect(actx.destination)
        const now  = actx.currentTime
        const when = Math.max(now, nextTimeRef.current)
        src.start(when)
        nextTimeRef.current = when + abuf.duration

      } else if (tag === TAG_SURFACE) {
        // Raw surface pixels from surface_render.rs — apply brightness boost so
        // low-drive states (pixel values 1-10/255) are visible at full screen scale.
        if (buf.byteLength < 5) return
        const w = view.getUint16(1, true)
        const h = view.getUint16(3, true)
        if (buf.byteLength < 5 + w * h * 4) return
        const cv = surfaceRef.current
        if (!cv) return
        const ctx2d = cv.getContext('2d')
        if (!ctx2d) return
        if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h }
        const src = new Uint8Array(buf.buffer, buf.byteOffset + 5, w * h * 4)
        const img = ctx2d.createImageData(w, h)
        const GAIN = 6  // lift low-drive pixels; saturates naturally at 255
        for (let i = 0; i < src.length; i += 4) {
          img.data[i]   = Math.min(255, src[i]   * GAIN)
          img.data[i+1] = Math.min(255, src[i+1] * GAIN)
          img.data[i+2] = Math.min(255, src[i+2] * GAIN)
          img.data[i+3] = 255
        }
        ctx2d.putImageData(img, 0, 0)
      }
    }

    const interval = setInterval(() => {
      if (ws.readyState !== WebSocket.OPEN) return
      const key = xorKeyRef.current
      if (!key) return
      const plain = new Uint8Array(packSenses(getSenses()))
      ws.send(xorApply(plain, key))
    }, 50)

    return () => {
      clearInterval(interval)
      ws.close()
      wsRef.current = null
      xorKeyRef.current = null
      actx.close()
      audioCtxRef.current = null
    }
  }, [running])
}

type Phase =
  | { k: 'requesting' }
  | { k: 'live' }
  | { k: 'denied' }
  | { k: 'unsupported' }

function useCallTimer(running: boolean) {
  const [secs, setSecs] = useState(0)
  useEffect(() => {
    if (!running) { setSecs(0); return }
    const id = setInterval(() => setSecs(s => s + 1), 1000)
    return () => clearInterval(id)
  }, [running])
  const m = String(Math.floor(secs / 60)).padStart(2, '0')
  const s = String(secs % 60).padStart(2, '0')
  return `${m}:${s}`
}

export function Commune({ onLeave }: { onLeave: () => void }) {
  const { t } = useI18n()
  const [phase, setPhase] = useState<Phase>({ k: 'requesting' })
  const [muted, setMuted] = useState(false)
  const [camOff, setCamOff] = useState(false)

  const perceptionRef  = useRef<Perception | null>(null)
  const streamRef      = useRef<MediaStream | null>(null)
  const selfVideoRef   = useRef<HTMLVideoElement>(null)
  const surfaceRef     = useRef<HTMLCanvasElement>(null)
  const fallback       = useRef(idleSenses())

  const timer = useCallTimer(phase.k === 'live')

  const begin = async () => {
    setPhase({ k: 'requesting' })
    const res: MediaResult = await acquireSenses()
    if (!res.ok) {
      setPhase({ k: res.reason === 'unsupported' ? 'unsupported' : 'denied' })
      return
    }
    streamRef.current = res.stream
    if (selfVideoRef.current) {
      selfVideoRef.current.srcObject = res.stream
      selfVideoRef.current.muted = true
      selfVideoRef.current.playsInline = true
      void selfVideoRef.current.play().catch(() => {})
    }
    const perc = new Perception()
    await perc.attach(res.stream)
    perc.resumeAudio()
    perceptionRef.current = perc
    setPhase({ k: 'live' })
  }

  useEffect(() => {
    void begin()
    return () => {
      perceptionRef.current?.dispose()
      releaseStream(streamRef.current)
      perceptionRef.current = null
      streamRef.current = null
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const leave = () => {
    perceptionRef.current?.dispose()
    releaseStream(streamRef.current)
    perceptionRef.current = null
    streamRef.current = null
    onLeave()
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

  useBodyBridge(phase.k === 'live', read, surfaceRef)

  const isLive = phase.k === 'live'

  return (
    <div className="commune">
      {/* Surface canvas — raw body visual from surface_render.rs */}
      <canvas ref={surfaceRef} className="commune__surface" aria-hidden="true" />

      {/* Top-left: minimal live indicator (no internal params) */}
      {isLive && (
        <div className="commune__remote-label fade-in">
          <span className="commune__remote-dot" />
          <span className="commune__remote-name">aborn</span>
          <span className="commune__timer">{timer}</span>
        </div>
      )}

      {/* Self-view PiP */}
      <div className={`commune__self ${isLive ? 'commune__self--visible' : ''} ${camOff ? 'commune__self--off' : ''}`}>
        <video ref={selfVideoRef} className="commune__self-video" muted playsInline autoPlay />
        {camOff && <div className="commune__self-off-icon"><CamOffIcon /></div>}
      </div>

      {/* Perception hint — nudge toward motion: depth & shape emerge from movement */}
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
            <button className="commune__btn" onClick={leave}>{t('commune.leave')}</button>
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
