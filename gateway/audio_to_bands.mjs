// audio_to_bands.mjs — offline PCM → 64-band mel frame transform.
//
// PURPOSE
//   Convert real speech audio files into the EXACT same {level, flux, bands[64]}
//   per-tick representation the browser's live perception path produces, so a
//   nursery body trained on real audio learns prototypes in the same coordinate
//   system the live (browser) path will later feed it.
//
//   This is a faithful offline replica of site/src/lib/perception.ts readAudio():
//     Blackman window → FFT(2048) → EMA smoothing(0.6) → dB[-100,-30] →
//     byte(0..255)/255 → 64 mel triangular bands (80–20000 Hz) × 1.4, clamp.
//   getByteFrequencyData semantics are reproduced (Web Audio AnalyserNode).
//
//   The downstream body reduces 64 bands → 16 VIB bands → 2 scalars
//   (spectral center = pitch_hint, high/low ratio = resonance_hint), so the
//   value of real audio is a RICH, VARIED trajectory of real phoneme spectra,
//   versus synthetic Gaussian bumps that collapse to one cluster.
//
// Zero external dependencies (pure JS FFT) — matches nursery_feeder's
// "runs anywhere" ethos.

// ── Constants (must match perception.ts) ──────────────────────────────────────
export const BANDS       = 64
export const FFT_SIZE    = 2048
export const NUM_BINS    = FFT_SIZE / 2          // 1024
export const MEL_LOW_HZ  = 80
export const MEL_HIGH_HZ  = 20000
export const SMOOTHING   = 0.6                   // AnalyserNode.smoothingTimeConstant
export const MIN_DB      = -100                  // AnalyserNode default
export const MAX_DB      = -30                   // AnalyserNode default
export const TARGET_SR   = 48000                 // canonical browser AudioContext rate
export const SEND_HZ     = 50                    // must match nursery_feeder DT
export const HOP         = Math.round(TARGET_SR / SEND_HZ)  // 960 samples = 20 ms

const clamp01 = (x) => (x < 0 ? 0 : x > 1 ? 1 : x)

// ── WAV decode ────────────────────────────────────────────────────────────────
// Parses a RIFF/WAVE buffer. Supports PCM 8/16/24/32-bit and IEEE float32.
// Returns { sampleRate, channels, samples: Float32Array } downmixed to mono [-1,1].
export function decodeWav(buf) {
  if (buf.length < 44) throw new Error('wav: too short')
  if (buf.toString('ascii', 0, 4) !== 'RIFF') throw new Error('wav: not RIFF')
  if (buf.toString('ascii', 8, 12) !== 'WAVE') throw new Error('wav: not WAVE')

  let off = 12
  let fmt = null
  let dataOff = -1
  let dataLen = 0
  while (off + 8 <= buf.length) {
    const id = buf.toString('ascii', off, off + 4)
    const size = buf.readUInt32LE(off + 4)
    const body = off + 8
    if (id === 'fmt ') {
      fmt = {
        audioFormat: buf.readUInt16LE(body),       // 1=PCM, 3=IEEE float
        channels:    buf.readUInt16LE(body + 2),
        sampleRate:  buf.readUInt32LE(body + 4),
        bitsPerSample: buf.readUInt16LE(body + 14),
      }
    } else if (id === 'data') {
      dataOff = body
      dataLen = Math.min(size, buf.length - body)
    }
    off = body + size + (size & 1) // chunks are word-aligned
  }
  if (!fmt) throw new Error('wav: no fmt chunk')
  if (dataOff < 0) throw new Error('wav: no data chunk')

  const { channels, sampleRate, bitsPerSample, audioFormat } = fmt
  const bytesPerSample = bitsPerSample / 8
  const frameBytes = bytesPerSample * channels
  const nFrames = Math.floor(dataLen / frameBytes)
  const samples = new Float32Array(nFrames)

  const readSample = (p) => {
    if (audioFormat === 3 && bitsPerSample === 32) return buf.readFloatLE(p)
    switch (bitsPerSample) {
      case 8:  return (buf.readUInt8(p) - 128) / 128            // unsigned 8-bit
      case 16: return buf.readInt16LE(p) / 32768
      case 24: {
        const b0 = buf[p], b1 = buf[p + 1], b2 = buf[p + 2]
        let v = b0 | (b1 << 8) | (b2 << 16)
        if (v & 0x800000) v |= ~0xffffff                       // sign-extend
        return v / 8388608
      }
      case 32: return buf.readInt32LE(p) / 2147483648
      default: throw new Error('wav: unsupported bits ' + bitsPerSample)
    }
  }

  for (let i = 0; i < nFrames; i++) {
    let acc = 0
    const base = dataOff + i * frameBytes
    for (let c = 0; c < channels; c++) acc += readSample(base + c * bytesPerSample)
    samples[i] = acc / channels // downmix to mono
  }
  return { sampleRate, channels, samples }
}

// ── Resample (linear interpolation) ───────────────────────────────────────────
export function resample(samples, fromSr, toSr) {
  if (fromSr === toSr) return samples
  const ratio = toSr / fromSr
  const n = Math.max(1, Math.round(samples.length * ratio))
  const out = new Float32Array(n)
  for (let i = 0; i < n; i++) {
    const srcPos = i / ratio
    const i0 = Math.floor(srcPos)
    const i1 = Math.min(samples.length - 1, i0 + 1)
    const frac = srcPos - i0
    out[i] = samples[i0] * (1 - frac) + samples[i1] * frac
  }
  return out
}

// ── Blackman window (Web Audio AnalyserNode default) ──────────────────────────
export function blackman(n) {
  const w = new Float32Array(n)
  const a0 = 0.42, a1 = 0.5, a2 = 0.08
  for (let i = 0; i < n; i++) {
    const x = (2 * Math.PI * i) / n
    w[i] = a0 - a1 * Math.cos(x) + a2 * Math.cos(2 * x)
  }
  return w
}

// ── Radix-2 iterative FFT (in-place, decimation-in-time) ──────────────────────
// re/im: Float64Array of length N (power of 2). Transforms in place.
export function fftInPlace(re, im) {
  const n = re.length
  if ((n & (n - 1)) !== 0) throw new Error('fft: length not power of 2')
  // bit-reversal permutation
  for (let i = 1, j = 0; i < n; i++) {
    let bit = n >> 1
    for (; j & bit; bit >>= 1) j ^= bit
    j ^= bit
    if (i < j) {
      const tr = re[i]; re[i] = re[j]; re[j] = tr
      const ti = im[i]; im[i] = im[j]; im[j] = ti
    }
  }
  for (let len = 2; len <= n; len <<= 1) {
    const ang = (-2 * Math.PI) / len
    const wr = Math.cos(ang), wi = Math.sin(ang)
    for (let i = 0; i < n; i += len) {
      let cr = 1, ci = 0
      for (let k = 0; k < len / 2; k++) {
        const ur = re[i + k],          ui = im[i + k]
        const vr = re[i + k + len / 2] * cr - im[i + k + len / 2] * ci
        const vi = re[i + k + len / 2] * ci + im[i + k + len / 2] * cr
        re[i + k] = ur + vr; im[i + k] = ui + vi
        re[i + k + len / 2] = ur - vr; im[i + k + len / 2] = ui - vi
        const ncr = cr * wr - ci * wi
        ci = cr * wi + ci * wr
        cr = ncr
      }
    }
  }
}

// ── Mel band bin boundaries (identical to perception.ts) ──────────────────────
function hzToMel(hz) { return 2595 * Math.log10(1 + hz / 700) }
function melToHz(mel) { return 700 * (Math.pow(10, mel / 2595) - 1) }

export function buildMelBandBoundaries(numBands, numBins, sampleRate) {
  const melLow = hzToMel(MEL_LOW_HZ)
  const melHigh = hzToMel(MEL_HIGH_HZ)
  const melPoints = []
  for (let i = 0; i <= numBands + 1; i++) {
    melPoints.push(melLow + (i / (numBands + 1)) * (melHigh - melLow))
  }
  const hzPerBin = sampleRate / (2 * numBins)
  return melPoints.map((mel) => Math.round(melToHz(mel) / hzPerBin))
}

// ── Analyser: stateful getByteFrequencyData replica ───────────────────────────
// Reproduces Web Audio AnalyserNode: windowed FFT magnitude (÷fftSize),
// EMA smoothing across hops, dB conversion, byte quantization.
export class Analyser {
  constructor() {
    this.win = blackman(FFT_SIZE)
    this.smoothed = new Float64Array(NUM_BINS) // EMA magnitude state
    this.re = new Float64Array(FFT_SIZE)
    this.im = new Float64Array(FFT_SIZE)
    this.bytes = new Uint8Array(NUM_BINS)
  }

  // window: Float32Array length FFT_SIZE (zero-padded at clip edges).
  // Returns this.bytes (Uint8Array[NUM_BINS]), reused each call.
  process(window) {
    const { re, im, win, smoothed, bytes } = this
    for (let i = 0; i < FFT_SIZE; i++) { re[i] = window[i] * win[i]; im[i] = 0 }
    fftInPlace(re, im)
    const span = MAX_DB - MIN_DB
    for (let k = 0; k < NUM_BINS; k++) {
      const mag = Math.sqrt(re[k] * re[k] + im[k] * im[k]) / FFT_SIZE
      const sm = SMOOTHING * smoothed[k] + (1 - SMOOTHING) * mag
      smoothed[k] = sm
      const db = sm > 0 ? 20 * Math.log10(sm) : -Infinity
      let b = Math.floor((255 * (db - MIN_DB)) / span)
      if (b < 0 || !Number.isFinite(b)) b = 0
      else if (b > 255) b = 255
      bytes[k] = b
    }
    return bytes
  }
}

// ── 64 mel bands from freq bytes (perception.ts mel loop) ─────────────────────
export function melBandsFromFreqBytes(freqBytes, melBins) {
  const bands = new Float32Array(BANDS)
  const numBins = freqBytes.length
  if (melBins.length !== BANDS + 2) return bands // safety: all-zero
  for (let b = 0; b < BANDS; b++) {
    const lo = melBins[b]
    const hi = melBins[b + 2]
    const center = melBins[b + 1]
    let bandEnergy = 0
    let bandCount = 0
    for (let i = lo; i < hi && i < numBins; i++) {
      let w
      if (i <= center) w = center > lo ? (i - lo) / (center - lo) : 1
      else             w = hi > center ? (hi - i) / (hi - center) : 1
      bandEnergy += (freqBytes[i] / 255) * w
      bandCount += w
    }
    bands[b] = bandCount > 0 ? clamp01((bandEnergy / bandCount) * 1.4) : 0
  }
  return bands
}

// ── Full transform: PCM samples → [{level, flux, bands[64]}] per 20 ms tick ───
// Matches perception.ts: level = clamp01(rms*3.2) from raw window,
// flux = onsetStrength = clamp01((sum positive band-byte diff)/numBins * 18).
export function audioToFrames(samples, sampleRate) {
  const pcm = resample(samples, sampleRate, TARGET_SR)
  const melBins = buildMelBandBoundaries(BANDS, NUM_BINS, TARGET_SR)
  const an = new Analyser()
  const prevFreq = new Float64Array(NUM_BINS)
  const win = new Float32Array(FFT_SIZE)
  const ticks = []

  const nTicks = Math.max(1, Math.ceil(pcm.length / HOP))
  for (let t = 0; t < nTicks; t++) {
    const start = t * HOP
    // fill window (zero-pad past end)
    let rms = 0
    for (let i = 0; i < FFT_SIZE; i++) {
      const s = start + i < pcm.length ? pcm[start + i] : 0
      win[i] = s
    }
    // RMS over the same window (time-domain), matching perception (rms*3.2).
    for (let i = 0; i < FFT_SIZE; i++) rms += win[i] * win[i]
    rms = Math.sqrt(rms / FFT_SIZE)
    const level = clamp01(rms * 3.2)

    const freqBytes = an.process(win)
    // onset flux from positive band-byte change
    let fluxPos = 0
    for (let k = 0; k < NUM_BINS; k++) {
      const v = freqBytes[k] / 255
      const d = v - prevFreq[k]
      if (d > 0) fluxPos += d
      prevFreq[k] = v
    }
    const flux = clamp01((fluxPos / NUM_BINS) * 18)
    const bands = melBandsFromFreqBytes(freqBytes, melBins)
    ticks.push({ level, flux, bands: Array.from(bands) })
  }
  return ticks
}

// ── Corpus binary (re)serialization ───────────────────────────────────────────
// Layout:
//   [4]  magic  "ACRP"
//   [1]  version 0x01
//   [4]  u32-le  n_utterances
//   per utterance:
//     [2]  u16-le label byte length
//     [..] label utf-8
//     [4]  u32-le n_ticks
//     per tick: [4] level f32 | [4] flux f32 | [64×4] bands f32
const CORPUS_MAGIC = 'ACRP'
const CORPUS_VERSION = 0x01

export function serializeCorpus(utterances) {
  const parts = []
  const head = Buffer.alloc(4 + 1 + 4)
  head.write(CORPUS_MAGIC, 0, 'ascii')
  head.writeUInt8(CORPUS_VERSION, 4)
  head.writeUInt32LE(utterances.length, 5)
  parts.push(head)
  for (const u of utterances) {
    const label = Buffer.from(u.label ?? '', 'utf8')
    const uh = Buffer.alloc(2 + label.length + 4)
    uh.writeUInt16LE(label.length, 0)
    label.copy(uh, 2)
    uh.writeUInt32LE(u.ticks.length, 2 + label.length)
    parts.push(uh)
    const body = Buffer.alloc(u.ticks.length * (4 + 4 + BANDS * 4))
    let o = 0
    for (const tk of u.ticks) {
      body.writeFloatLE(tk.level, o); o += 4
      body.writeFloatLE(tk.flux, o);  o += 4
      for (let b = 0; b < BANDS; b++) { body.writeFloatLE(tk.bands[b] ?? 0, o); o += 4 }
    }
    parts.push(body)
  }
  return Buffer.concat(parts)
}

export function deserializeCorpus(buf) {
  if (buf.length < 9) return null
  if (buf.toString('ascii', 0, 4) !== CORPUS_MAGIC) return null
  if (buf.readUInt8(4) !== CORPUS_VERSION) return null
  const n = buf.readUInt32LE(5)
  let off = 9
  const utterances = []
  const TICK = 4 + 4 + BANDS * 4
  for (let u = 0; u < n; u++) {
    if (off + 2 > buf.length) return null
    const labelLen = buf.readUInt16LE(off); off += 2
    if (off + labelLen + 4 > buf.length) return null
    const label = buf.toString('utf8', off, off + labelLen); off += labelLen
    const nTicks = buf.readUInt32LE(off); off += 4
    if (off + nTicks * TICK > buf.length) return null
    const ticks = []
    for (let t = 0; t < nTicks; t++) {
      const level = buf.readFloatLE(off); off += 4
      const flux  = buf.readFloatLE(off); off += 4
      const bands = new Array(BANDS)
      for (let b = 0; b < BANDS; b++) { bands[b] = buf.readFloatLE(off); off += 4 }
      ticks.push({ level, flux, bands })
    }
    utterances.push({ label, ticks })
  }
  return utterances
}

// ── Helpers for tests / tooling ───────────────────────────────────────────────
// Energy-weighted spectral center of a bands[64] array → [0,1] band-index ratio.
// This is what the body reduces to pitch_hint (via VIB downsample).
export function spectralCenter(bands) {
  let num = 0, den = 0
  for (let i = 0; i < bands.length; i++) { num += bands[i] * (i / (bands.length - 1)); den += bands[i] }
  return den > 0 ? num / den : 0
}
