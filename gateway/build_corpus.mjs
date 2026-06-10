#!/usr/bin/env node
// build_corpus.mjs — offline: a directory of WAV clips → corpus_frames.bin
//
//   node build_corpus.mjs --in <wav_dir> --out <corpus_frames.bin> [--peak 0.7]
//
// Each .wav becomes one labelled utterance (label = filename without extension).
// Clips are peak-normalized so their level lands in the gentle ~0.3–0.6 regime
// the nursery curriculum uses (vital exertion is per-voicing-tick, not per
// magnitude — duty cycle is the survival lever, so we keep clips comparable).
// The heavy mel transform runs HERE (offline, tested), not in the live feeder.

import { readdirSync, readFileSync, writeFileSync, statSync } from 'node:fs'
import { join, basename, extname } from 'node:path'
import { decodeWav, audioToFrames, serializeCorpus, HOP, TARGET_SR } from './audio_to_bands.mjs'

function arg(name, def) {
  const i = process.argv.indexOf('--' + name)
  return i >= 0 && i + 1 < process.argv.length ? process.argv[i + 1] : def
}

const IN_DIR  = arg('in', null)
const OUT     = arg('out', null)
const PEAK    = parseFloat(arg('peak', '0.7'))

if (!IN_DIR || !OUT) {
  console.error('usage: node build_corpus.mjs --in <wav_dir> --out <corpus.bin> [--peak 0.7]')
  process.exit(1)
}

// Peak-normalize a Float32Array in place toward target peak amplitude.
function peakNormalize(samples, target) {
  let peak = 0
  for (let i = 0; i < samples.length; i++) { const a = Math.abs(samples[i]); if (a > peak) peak = a }
  if (peak < 1e-6) return samples
  const g = target / peak
  for (let i = 0; i < samples.length; i++) samples[i] *= g
  return samples
}

const files = readdirSync(IN_DIR)
  .filter((f) => ['.wav', '.wave'].includes(extname(f).toLowerCase()))
  .filter((f) => { try { return statSync(join(IN_DIR, f)).isFile() } catch { return false } })
  .sort()

if (!files.length) { console.error(`no .wav files in ${IN_DIR}`); process.exit(1) }

const utterances = []
let totalTicks = 0
for (const f of files) {
  try {
    const { sampleRate, samples } = decodeWav(readFileSync(join(IN_DIR, f)))
    peakNormalize(samples, PEAK)
    const ticks = audioToFrames(samples, sampleRate)
    const label = basename(f, extname(f))
    utterances.push({ label, ticks })
    totalTicks += ticks.length
    const durMs = Math.round((ticks.length * HOP / TARGET_SR) * 1000)
    console.log(`  ${label.padEnd(16)} ${String(durMs).padStart(5)}ms  ${ticks.length} ticks  (src ${sampleRate}Hz)`)
  } catch (e) {
    console.warn(`  SKIP ${f}: ${e.message}`)
  }
}

const bin = serializeCorpus(utterances)
writeFileSync(OUT, bin)
console.log(`\nwrote ${OUT}: ${utterances.length} utterances, ${totalTicks} ticks, ${(bin.length / 1024).toFixed(1)} KB`)
console.log(`total voiced time ≈ ${(totalTicks * HOP / TARGET_SR).toFixed(1)}s`)
