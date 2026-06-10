// Tests for audio_to_bands.mjs — run: node --test audio_to_bands.test.mjs
import test from 'node:test'
import assert from 'node:assert/strict'
import {
  BANDS, FFT_SIZE, NUM_BINS, TARGET_SR, HOP,
  decodeWav, resample, blackman, fftInPlace, buildMelBandBoundaries,
  Analyser, melBandsFromFreqBytes, audioToFrames,
  serializeCorpus, deserializeCorpus, spectralCenter,
} from './audio_to_bands.mjs'

// ── helper: build a 16-bit PCM mono/stereo WAV buffer from Float32 channel(s) ──
function makeWav(channelData, sampleRate) {
  const channels = channelData.length
  const nFrames = channelData[0].length
  const bytesPerSample = 2
  const dataLen = nFrames * channels * bytesPerSample
  const buf = Buffer.alloc(44 + dataLen)
  buf.write('RIFF', 0, 'ascii')
  buf.writeUInt32LE(36 + dataLen, 4)
  buf.write('WAVE', 8, 'ascii')
  buf.write('fmt ', 12, 'ascii')
  buf.writeUInt32LE(16, 16)             // fmt chunk size
  buf.writeUInt16LE(1, 20)              // PCM
  buf.writeUInt16LE(channels, 22)
  buf.writeUInt32LE(sampleRate, 24)
  buf.writeUInt32LE(sampleRate * channels * bytesPerSample, 28) // byte rate
  buf.writeUInt16LE(channels * bytesPerSample, 32)              // block align
  buf.writeUInt16LE(16, 34)            // bits per sample
  buf.write('data', 36, 'ascii')
  buf.writeUInt32LE(dataLen, 40)
  let o = 44
  for (let i = 0; i < nFrames; i++) {
    for (let c = 0; c < channels; c++) {
      let v = Math.round(channelData[c][i] * 32767)
      v = Math.max(-32768, Math.min(32767, v))
      buf.writeInt16LE(v, o); o += 2
    }
  }
  return buf
}

function sine(freq, durSec, sampleRate, amp = 0.8) {
  const n = Math.round(durSec * sampleRate)
  const s = new Float32Array(n)
  for (let i = 0; i < n; i++) s[i] = amp * Math.sin(2 * Math.PI * freq * i / sampleRate)
  return s
}

// ── TDD-A1: WAV decode round-trip ─────────────────────────────────────────────
test('A1 decodeWav: 16-bit mono round-trips within quantization', () => {
  const src = sine(440, 0.05, 16000, 0.5)
  const wav = makeWav([src], 16000)
  const { sampleRate, channels, samples } = decodeWav(wav)
  assert.equal(sampleRate, 16000)
  assert.equal(channels, 1)
  assert.equal(samples.length, src.length)
  let maxErr = 0
  for (let i = 0; i < src.length; i++) maxErr = Math.max(maxErr, Math.abs(samples[i] - src[i]))
  assert.ok(maxErr < 1 / 32000, `quantization error too large: ${maxErr}`)
})

test('A1b decodeWav: stereo downmixes to mono', () => {
  const left  = new Float32Array([0.5, 0.5, 0.5, 0.5])
  const right = new Float32Array([-0.5, -0.5, -0.5, -0.5]) // L+R should cancel → ~0
  const wav = makeWav([left, right], 16000)
  const { channels, samples } = decodeWav(wav)
  assert.equal(channels, 2)
  assert.equal(samples.length, 4)
  for (const s of samples) assert.ok(Math.abs(s) < 1 / 32000, `downmix not ~0: ${s}`)
})

test('A1c decodeWav: rejects garbage', () => {
  assert.throws(() => decodeWav(Buffer.from('not a wav file at all!!')))
})

// ── TDD-A2: Blackman window ───────────────────────────────────────────────────
test('A2 blackman: endpoints near zero, center near one, symmetric', () => {
  const w = blackman(FFT_SIZE)
  assert.equal(w.length, FFT_SIZE)
  assert.ok(w[0] < 0.01, `endpoint not ~0: ${w[0]}`)
  assert.ok(w[FFT_SIZE / 2] > 0.9, `center not ~1: ${w[FFT_SIZE / 2]}`)
  for (let i = 1; i < FFT_SIZE / 2; i++) {
    assert.ok(Math.abs(w[i] - w[FFT_SIZE - i]) < 1e-9, `not symmetric at ${i}`)
  }
})

// ── TDD-A3: FFT correctness ───────────────────────────────────────────────────
test('A3 fft: pure cosine concentrates energy at its bin', () => {
  const N = 64
  const bin = 5
  const re = new Float64Array(N)
  const im = new Float64Array(N)
  for (let i = 0; i < N; i++) re[i] = Math.cos(2 * Math.PI * bin * i / N)
  fftInPlace(re, im)
  const mag = (k) => Math.sqrt(re[k] * re[k] + im[k] * im[k])
  // peak should be at `bin` (and its conjugate N-bin); check bin dominates neighbors
  assert.ok(mag(bin) > mag(bin - 1) * 5, 'peak not at expected bin')
  assert.ok(mag(bin) > mag(bin + 1) * 5, 'peak not at expected bin')
  assert.ok(mag(bin) > mag(10), 'peak not dominant')
})

test('A3b fft: rejects non-power-of-2', () => {
  assert.throws(() => fftInPlace(new Float64Array(3), new Float64Array(3)))
})

// ── TDD-A4: mel band boundaries ───────────────────────────────────────────────
test('A4 melBins: monotonic, correct length, within bin range', () => {
  const bins = buildMelBandBoundaries(BANDS, NUM_BINS, TARGET_SR)
  assert.equal(bins.length, BANDS + 2)
  for (let i = 1; i < bins.length; i++) {
    assert.ok(bins[i] >= bins[i - 1], `not monotonic at ${i}: ${bins[i - 1]}→${bins[i]}`)
  }
  assert.ok(bins[0] >= 0)
  assert.ok(bins[bins.length - 1] <= NUM_BINS, `last bin out of range: ${bins[bins.length - 1]}`)
})

// ── TDD-A5: dB conversion + byte quantization ─────────────────────────────────
test('A5 Analyser: silence → all zero bytes', () => {
  const an = new Analyser()
  const bytes = an.process(new Float32Array(FFT_SIZE)) // all zeros
  for (const b of bytes) assert.equal(b, 0)
})

test('A5b Analyser: loud full-scale tone → high (clamped) bytes at its bin', () => {
  const an = new Analyser()
  const win = new Float32Array(FFT_SIZE)
  const bin = 40
  // Feed several hops so EMA settles toward the true magnitude.
  let bytes
  for (let rep = 0; rep < 30; rep++) {
    for (let i = 0; i < FFT_SIZE; i++) win[i] = Math.sin(2 * Math.PI * bin * i / FFT_SIZE)
    bytes = an.process(win)
  }
  assert.ok(bytes[bin] > 200, `tone bin should be high after settling: ${bytes[bin]}`)
  assert.ok(bytes[bin] <= 255)
  // far-away bin should be much lower
  assert.ok(bytes[bin] > bytes[bin + 200] + 50, 'tone bin not dominant over distant bin')
})

// ── TDD-A6: end-to-end — spectral center tracks real pitch ────────────────────
test('A6 audioToFrames: higher pitch → higher spectral center', () => {
  const low  = audioToFrames(sine(300,  0.4, TARGET_SR), TARGET_SR)
  const high = audioToFrames(sine(4000, 0.4, TARGET_SR), TARGET_SR)
  // average spectral center over the steady middle of each clip
  const avgCenter = (ticks) => {
    const mid = ticks.slice(Math.floor(ticks.length * 0.4), Math.floor(ticks.length * 0.9))
    const cs = mid.map((t) => spectralCenter(t.bands)).filter((c) => c > 0)
    return cs.reduce((a, b) => a + b, 0) / Math.max(1, cs.length)
  }
  const cLow = avgCenter(low)
  const cHigh = avgCenter(high)
  assert.ok(cLow > 0 && cHigh > 0, `degenerate centers: ${cLow}, ${cHigh}`)
  assert.ok(cHigh > cLow + 0.05, `4kHz center (${cHigh}) should exceed 300Hz center (${cLow})`)
})

// ── TDD-A7: silence frames ────────────────────────────────────────────────────
test('A7 audioToFrames: silence → zero level and zero bands', () => {
  const ticks = audioToFrames(new Float32Array(TARGET_SR * 0.2), TARGET_SR) // 200ms silence
  for (const t of ticks) {
    assert.equal(t.level, 0)
    for (const b of t.bands) assert.equal(b, 0)
  }
})

// ── TDD-A8: band range + frame shape ──────────────────────────────────────────
test('A8 audioToFrames: bands always [0,1] and length 64', () => {
  const ticks = audioToFrames(sine(1000, 0.2, TARGET_SR), TARGET_SR)
  for (const t of ticks) {
    assert.equal(t.bands.length, BANDS)
    assert.ok(t.level >= 0 && t.level <= 1)
    assert.ok(t.flux >= 0 && t.flux <= 1)
    for (const b of t.bands) assert.ok(b >= 0 && b <= 1, `band out of range: ${b}`)
  }
})

// ── TDD-A10: tick cadence matches 20ms hop ────────────────────────────────────
test('A10 audioToFrames: ~1s clip yields ~50 ticks (20ms each)', () => {
  const ticks = audioToFrames(sine(440, 1.0, TARGET_SR), TARGET_SR)
  // 48000 samples / 960 hop = 50
  assert.ok(Math.abs(ticks.length - 50) <= 1, `expected ~50 ticks, got ${ticks.length}`)
})

test('A10b audioToFrames: resamples non-48k input to 48k cadence', () => {
  const ticks = audioToFrames(sine(440, 1.0, 16000), 16000) // 1s @ 16k → resample to 48k
  assert.ok(Math.abs(ticks.length - 50) <= 1, `expected ~50 ticks, got ${ticks.length}`)
})

// ── TDD-A9: corpus binary round-trip ──────────────────────────────────────────
test('A9 corpus serialize/deserialize round-trip', () => {
  const utterances = [
    { label: 'a', ticks: audioToFrames(sine(500, 0.1, TARGET_SR), TARGET_SR) },
    { label: '妈妈', ticks: audioToFrames(sine(900, 0.08, TARGET_SR), TARGET_SR) },
    { label: '', ticks: [] }, // empty edge case
  ]
  const bin = serializeCorpus(utterances)
  const back = deserializeCorpus(bin)
  assert.ok(back, 'deserialize returned null')
  assert.equal(back.length, utterances.length)
  for (let u = 0; u < utterances.length; u++) {
    assert.equal(back[u].label, utterances[u].label)
    assert.equal(back[u].ticks.length, utterances[u].ticks.length)
    for (let t = 0; t < utterances[u].ticks.length; t++) {
      const a = utterances[u].ticks[t], b = back[u].ticks[t]
      assert.ok(Math.abs(a.level - b.level) < 1e-6)
      assert.ok(Math.abs(a.flux - b.flux) < 1e-6)
      for (let i = 0; i < BANDS; i++) assert.ok(Math.abs(a.bands[i] - b.bands[i]) < 1e-6)
    }
  }
})

test('A9b corpus deserialize: rejects corrupt data', () => {
  assert.equal(deserializeCorpus(Buffer.alloc(3)), null)
  assert.equal(deserializeCorpus(Buffer.from('JUNK56789')), null)
  const good = serializeCorpus([{ label: 'x', ticks: [] }])
  assert.equal(deserializeCorpus(good.subarray(0, good.length - 1)), null)
})
