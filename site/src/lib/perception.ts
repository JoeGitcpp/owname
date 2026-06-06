// Perception — turns the raw camera + mic streams into a small bundle of
// tier-0-ish signals the modality renderer integrates over. Nothing here is
// stored or transmitted; every read() is computed on the live frame.

export interface Senses {
  audioLevel: number      // RMS amplitude, 0..1
  audioFlux: number       // positive spectral change (onset energy), 0..1
  bands: Float32Array     // 64 mel-scale frequency bands, each 0..1
  motion: number          // mean inter-frame luminance change, 0..1
  luma: number            // mean brightness, 0..1
  hue: number             // dominant hue, 0..1
  gw: number
  gh: number
  grid: Float32Array      // downsampled luminance grid, gw*gh, 0..1 (backward compat)
  gridR: Float32Array     // 160×120 red channel, 0..1
  gridG: Float32Array     // 160×120 green channel, 0..1
  gridB: Float32Array     // 160×120 blue channel, 0..1
  // aux audio features
  spectralCentroid: number
  spectralFlatness: number
  spectralRolloff: number
  spectralSpread: number
  zcr: number
  hnr: number
  onsetStrength: number
  temporalFlux: number
  hasVideo: boolean
}

const GW = 160
const GH = 120
const BANDS = 64
const GRID_CHANNELS = 3
const GRID_TOTAL = GW * GH * GRID_CHANNELS
const FFT_SIZE = 2048

// Mel scale helpers
const MEL_LOW_HZ = 80
const MEL_HIGH_HZ = 20000

function hzToMel(hz: number): number {
  return 2595 * Math.log10(1 + hz / 700)
}

function melToHz(mel: number): number {
  return 700 * (Math.pow(10, mel / 2595) - 1)
}

// Precompute mel band bin boundaries once
function buildMelBandBoundaries(numBands: number, numBins: number, sampleRate: number): number[] {
  const melLow = hzToMel(MEL_LOW_HZ)
  const melHigh = hzToMel(MEL_HIGH_HZ)
  // numBands+2 mel points evenly spaced
  const melPoints: number[] = []
  for (let i = 0; i <= numBands + 1; i++) {
    melPoints.push(melLow + (i / (numBands + 1)) * (melHigh - melLow))
  }
  // Convert mel points to bin indices
  const hzPerBin = sampleRate / (2 * numBins)
  return melPoints.map(mel => Math.round(melToHz(mel) / hzPerBin))
}

export class Perception {
  private video: HTMLVideoElement
  private sample: HTMLCanvasElement
  private sctx: CanvasRenderingContext2D
  private prevGrid: Float32Array    // luma for motion
  private gridLuma: Float32Array
  private gridR: Float32Array
  private gridG: Float32Array
  private gridB: Float32Array

  private audioCtx: AudioContext | null = null
  private analyser: AnalyserNode | null = null
  private freq: Uint8Array<ArrayBuffer> = new Uint8Array(new ArrayBuffer(0))
  private prevFreq: Float32Array = new Float32Array(0)
  private time: Uint8Array<ArrayBuffer> = new Uint8Array(new ArrayBuffer(0))

  private melBins: number[] = []  // populated after audioCtx is created
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
    this.gridLuma = new Float32Array(GW * GH)
    this.gridR = new Float32Array(GW * GH)
    this.gridG = new Float32Array(GW * GH)
    this.gridB = new Float32Array(GW * GH)
    this.senses = {
      audioLevel: 0,
      audioFlux: 0,
      bands: new Float32Array(BANDS),
      motion: 0,
      luma: 0,
      hue: 0.5,
      gw: GW,
      gh: GH,
      grid: this.gridLuma,
      gridR: this.gridR,
      gridG: this.gridG,
      gridB: this.gridB,
      spectralCentroid: 0,
      spectralFlatness: 0,
      spectralRolloff: 0,
      spectralSpread: 0,
      zcr: 0,
      hnr: 0.5,
      onsetStrength: 0,
      temporalFlux: 0,
      hasVideo: false,
    }
  }

  async attach(stream: MediaStream) {
    // video
    if (stream.getVideoTracks().length) {
      this.video.srcObject = stream
      // Mobile browsers (especially iOS Safari) refuse to decode MediaStream
      // frames for canvas.drawImage() when the video element is off-DOM.
      // Pin a 1×1 invisible element to the body so the browser treats it as
      // "visible" and delivers real frames.  Removed in dispose().
      if (!this.video.parentNode) {
        this.video.style.cssText =
          'position:fixed;top:-1px;left:-1px;width:1px;height:1px;opacity:0.001;pointer-events:none;z-index:-9999'
        document.body.appendChild(this.video)
      }
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
      this.analyser.fftSize = FFT_SIZE
      this.analyser.smoothingTimeConstant = 0.6
      src.connect(this.analyser)
      const n = this.analyser.frequencyBinCount  // 1024
      this.freq = new Uint8Array(new ArrayBuffer(n))
      this.prevFreq = new Float32Array(n)
      this.time = new Uint8Array(new ArrayBuffer(this.analyser.fftSize))
      this.melBins = buildMelBandBoundaries(BANDS, n, this.audioCtx.sampleRate)
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
    const luma = this.gridLuma
    const r = this.gridR
    const g = this.gridG
    const b = this.gridB
    const pg = this.prevGrid
    const nPixels = GW * GH
    for (let i = 0, p = 0; i < nPixels; i++, p += 4) {
      const rv = img[p] / 255
      const gv = img[p + 1] / 255
      const bv = img[p + 2] / 255
      r[i] = rv
      g[i] = gv
      b[i] = bv
      const l = 0.299 * rv + 0.587 * gv + 0.114 * bv
      luma[i] = l
      lumaSum += l
      motion += Math.abs(l - pg[i])
      // saturation-weighted hue
      const maxC = Math.max(rv, gv, bv)
      const minC = Math.min(rv, gv, bv)
      const d = maxC - minC
      if (d > 0.04) {
        let h = 0
        if (maxC === rv) h = ((gv - bv) / d) % 6
        else if (maxC === gv) h = (bv - rv) / d + 2
        else h = (rv - gv) / d + 4
        h /= 6
        if (h < 0) h += 1
        const a = h * Math.PI * 2
        const w = d
        hx += Math.cos(a) * w
        hy += Math.sin(a) * w
        hueWeight += w
      }
    }
    pg.set(luma)
    this.senses.motion = clamp01((motion / nPixels) * 6)
    this.senses.luma = lumaSum / nPixels
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

    // --- RMS from time domain ---
    let rmsSum = 0
    const timeLen = this.time.length
    for (let i = 0; i < timeLen; i++) {
      const s = (this.time[i] - 128) / 128
      rmsSum += s * s
    }
    const rms = Math.sqrt(rmsSum / timeLen)
    this.senses.audioLevel = clamp01(rms * 3.2)

    // --- Zero crossing rate ---
    let zcrCount = 0
    for (let i = 1; i < timeLen; i++) {
      const prev = this.time[i - 1] - 128
      const curr = this.time[i] - 128
      if ((prev >= 0 && curr < 0) || (prev < 0 && curr >= 0)) zcrCount++
    }
    this.senses.zcr = clamp01(zcrCount / (timeLen - 1) * 4)

    // --- Spectral features from frequency domain ---
    const f = this.freq
    const pf = this.prevFreq
    const numBins = f.length  // 1024
    let totalEnergy = 0
    let fluxPos = 0
    let fluxTotal = 0
    const freqVals = new Float32Array(numBins)

    for (let i = 0; i < numBins; i++) {
      const v = f[i] / 255
      freqVals[i] = v
      totalEnergy += v
      const d = v - pf[i]
      if (d > 0) fluxPos += d
      fluxTotal += Math.abs(v - pf[i])
      pf[i] = v
    }

    // --- 64 mel-scale bands ---
    const bands = this.senses.bands
    bands.fill(0)
    const melBins = this.melBins
    if (melBins.length === BANDS + 2) {
      for (let b = 0; b < BANDS; b++) {
        const lo = melBins[b]
        const hi = melBins[b + 2]
        const center = melBins[b + 1]
        let bandEnergy = 0
        let bandCount = 0
        for (let i = lo; i < hi && i < numBins; i++) {
          // triangular filter weight
          let w: number
          if (i <= center) {
            w = center > lo ? (i - lo) / (center - lo) : 1
          } else {
            w = hi > center ? (hi - i) / (hi - center) : 1
          }
          bandEnergy += freqVals[i] * w
          bandCount += w
        }
        bands[b] = bandCount > 0 ? clamp01((bandEnergy / bandCount) * 1.4) : 0
      }
    } else {
      // fallback: linear bands (before audioCtx is ready)
      const per = numBins / BANDS
      for (let i = 0; i < numBins; i++) {
        const b = Math.min(BANDS - 1, Math.floor(i / per))
        bands[b] += freqVals[i]
      }
      const per2 = numBins / BANDS
      for (let b = 0; b < BANDS; b++) bands[b] = clamp01((bands[b] / per2) * 1.4)
    }

    // --- Spectral centroid & spread ---
    let weightedFreqSum = 0
    let energySum = 0
    for (let i = 0; i < numBins; i++) {
      weightedFreqSum += i * freqVals[i]
      energySum += freqVals[i]
    }
    const centroidBin = energySum > 0 ? weightedFreqSum / energySum : 0
    this.senses.spectralCentroid = clamp01(centroidBin / numBins)

    let spreadSum = 0
    for (let i = 0; i < numBins; i++) {
      const diff = i - centroidBin
      spreadSum += diff * diff * freqVals[i]
    }
    const spread = energySum > 0 ? Math.sqrt(spreadSum / energySum) : 0
    this.senses.spectralSpread = clamp01(spread / numBins)

    // --- Spectral rolloff (85% of energy) ---
    const rolloffThreshold = energySum * 0.85
    let cumEnergy = 0
    let rolloffBin = numBins - 1
    for (let i = 0; i < numBins; i++) {
      cumEnergy += freqVals[i]
      if (cumEnergy >= rolloffThreshold) {
        rolloffBin = i
        break
      }
    }
    this.senses.spectralRolloff = clamp01(rolloffBin / numBins)

    // --- Spectral flatness (geometric mean / arithmetic mean) ---
    let logSum = 0
    let arithmeticSum = 0
    let nonZeroCount = 0
    for (let i = 0; i < numBins; i++) {
      if (freqVals[i] > 1e-10) {
        logSum += Math.log(freqVals[i])
        nonZeroCount++
      }
      arithmeticSum += freqVals[i]
    }
    if (nonZeroCount > 0 && arithmeticSum > 0) {
      const geomMean = Math.exp(logSum / nonZeroCount)
      const arithMean = arithmeticSum / numBins
      this.senses.spectralFlatness = clamp01(geomMean / arithMean)
    } else {
      this.senses.spectralFlatness = 0
    }

    // --- Onset strength & temporal flux ---
    this.senses.onsetStrength = clamp01((fluxPos / numBins) * 18)
    this.senses.temporalFlux = clamp01((fluxTotal / numBins) * 12)
    this.senses.audioFlux = this.senses.onsetStrength  // keep backward compat alias

    // --- HNR: approximation without pitch tracking ---
    this.senses.hnr = 0.5
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
    if (this.video.parentNode) this.video.parentNode.removeChild(this.video)
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

// Exported for tree-shaking awareness; not used internally but available to consumers.
export const PERCEPTION_GRID_CHANNELS = GRID_CHANNELS
export const PERCEPTION_GRID_TOTAL = GRID_TOTAL
