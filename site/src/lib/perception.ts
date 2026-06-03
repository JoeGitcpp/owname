// Perception — turns the raw camera + mic streams into a small bundle of
// tier-0-ish signals the modality renderer integrates over. Nothing here is
// stored or transmitted; every read() is computed on the live frame.

export interface Senses {
  audioLevel: number // RMS amplitude, 0..1
  audioFlux: number // positive spectral change (onset energy), 0..1
  bands: Float32Array // coarse frequency bands, each 0..1
  motion: number // mean inter-frame luminance change, 0..1
  luma: number // mean brightness, 0..1
  hue: number // dominant hue, 0..1
  gw: number
  gh: number
  grid: Float32Array // downsampled luminance grid, gw*gh, 0..1
  hasVideo: boolean
}

const GW = 32
const GH = 24
const BANDS = 12

export class Perception {
  private video: HTMLVideoElement
  private sample: HTMLCanvasElement
  private sctx: CanvasRenderingContext2D
  private prevGrid: Float32Array
  private grid: Float32Array

  private audioCtx: AudioContext | null = null
  private analyser: AnalyserNode | null = null
  private freq: Uint8Array<ArrayBuffer> = new Uint8Array(new ArrayBuffer(0))
  private prevFreq: Float32Array = new Float32Array(0)
  private time: Uint8Array<ArrayBuffer> = new Uint8Array(new ArrayBuffer(0))

  private senses: Senses

  constructor() {
    this.video = document.createElement('video')
    this.video.muted = true
    this.video.playsInline = true
    this.sample = document.createElement('canvas')
    this.sample.width = GW
    this.sample.height = GH
    const ctx = this.sample.getContext('2d', { willReadFrequently: true })
    if (!ctx) throw new Error('2d context unavailable')
    this.sctx = ctx
    this.prevGrid = new Float32Array(GW * GH)
    this.grid = new Float32Array(GW * GH)
    this.senses = {
      audioLevel: 0,
      audioFlux: 0,
      bands: new Float32Array(BANDS),
      motion: 0,
      luma: 0,
      hue: 0.5,
      gw: GW,
      gh: GH,
      grid: this.grid,
      hasVideo: false,
    }
  }

  async attach(stream: MediaStream) {
    // video
    if (stream.getVideoTracks().length) {
      this.video.srcObject = stream
      try {
        await this.video.play()
      } catch {
        /* autoplay may defer; frames still flow once playing */
      }
    }
    // audio
    if (stream.getAudioTracks().length) {
      const Ctor =
        window.AudioContext ||
        (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext
      this.audioCtx = new Ctor()
      const src = this.audioCtx.createMediaStreamSource(stream)
      this.analyser = this.audioCtx.createAnalyser()
      this.analyser.fftSize = 1024
      this.analyser.smoothingTimeConstant = 0.6
      src.connect(this.analyser)
      const n = this.analyser.frequencyBinCount
      this.freq = new Uint8Array(new ArrayBuffer(n))
      this.prevFreq = new Float32Array(n)
      this.time = new Uint8Array(new ArrayBuffer(this.analyser.fftSize))
    }
  }

  private readVideo() {
    const v = this.video
    if (!v.videoWidth || v.readyState < 2) return
    this.senses.hasVideo = true
    // Mirror horizontally so motion maps intuitively to the viewer.
    this.sctx.save()
    this.sctx.translate(GW, 0)
    this.sctx.scale(-1, 1)
    this.sctx.drawImage(v, 0, 0, GW, GH)
    this.sctx.restore()
    const img = this.sctx.getImageData(0, 0, GW, GH).data

    let motion = 0
    let lumaSum = 0
    // hue accumulation via vector sum on the color wheel
    let hx = 0
    let hy = 0
    let hueWeight = 0
    const g = this.grid
    const pg = this.prevGrid
    for (let i = 0, p = 0; i < g.length; i++, p += 4) {
      const r = img[p] / 255
      const gr = img[p + 1] / 255
      const b = img[p + 2] / 255
      const l = 0.299 * r + 0.587 * gr + 0.114 * b
      g[i] = l
      lumaSum += l
      motion += Math.abs(l - pg[i])
      // saturation-weighted hue
      const max = Math.max(r, gr, b)
      const min = Math.min(r, gr, b)
      const d = max - min
      if (d > 0.04) {
        let h = 0
        if (max === r) h = ((gr - b) / d) % 6
        else if (max === gr) h = (b - r) / d + 2
        else h = (r - gr) / d + 4
        h /= 6
        if (h < 0) h += 1
        const a = h * Math.PI * 2
        const w = d
        hx += Math.cos(a) * w
        hy += Math.sin(a) * w
        hueWeight += w
      }
    }
    pg.set(g)
    const n = g.length
    this.senses.motion = clamp01((motion / n) * 6)
    this.senses.luma = lumaSum / n
    if (hueWeight > 0.001) {
      let ha = Math.atan2(hy, hx) / (Math.PI * 2)
      if (ha < 0) ha += 1
      // ease hue toward the new estimate to avoid flicker
      this.senses.hue = lerpAngle(this.senses.hue, ha, 0.08)
    }
  }

  private readAudio() {
    const an = this.analyser
    if (!an) return
    an.getByteFrequencyData(this.freq)
    an.getByteTimeDomainData(this.time)
    // RMS from time domain
    let sum = 0
    for (let i = 0; i < this.time.length; i++) {
      const s = (this.time[i] - 128) / 128
      sum += s * s
    }
    const rms = Math.sqrt(sum / this.time.length)
    this.senses.audioLevel = clamp01(rms * 3.2)

    // spectral flux (only positive changes) + coarse bands
    const f = this.freq
    const pf = this.prevFreq
    let flux = 0
    const bands = this.senses.bands
    bands.fill(0)
    const per = f.length / BANDS
    for (let i = 0; i < f.length; i++) {
      const v = f[i] / 255
      const d = v - pf[i]
      if (d > 0) flux += d
      pf[i] = v
      const b = Math.min(BANDS - 1, Math.floor(i / per))
      bands[b] += v
    }
    for (let b = 0; b < BANDS; b++) bands[b] = clamp01((bands[b] / per) * 1.4)
    this.senses.audioFlux = clamp01((flux / f.length) * 18)
  }

  read(): Senses {
    this.readVideo()
    this.readAudio()
    return this.senses
  }

  // Some browsers suspend the AudioContext until a user gesture; the Commune
  // entry is a click, so resume there.
  resumeAudio() {
    if (this.audioCtx && this.audioCtx.state === 'suspended') {
      void this.audioCtx.resume()
    }
  }

  dispose() {
    this.video.srcObject = null
    if (this.audioCtx) {
      void this.audioCtx.close()
      this.audioCtx = null
    }
  }
}

function clamp01(x: number) {
  return x < 0 ? 0 : x > 1 ? 1 : x
}

// shortest-path interpolation around the [0,1) hue circle
function lerpAngle(a: number, b: number, t: number) {
  let d = b - a
  if (d > 0.5) d -= 1
  if (d < -0.5) d += 1
  let r = a + d * t
  if (r < 0) r += 1
  if (r >= 1) r -= 1
  return r
}
