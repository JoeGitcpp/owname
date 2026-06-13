/**
 * nursery_feeder.mjs — continuous vocal curriculum feeder for a headless body.
 *
 * Problem this solves:
 *   On the server the body only hears random browser visitors for ≤30 min, then
 *   the session ends. The vocal-imitation loop needs the SAME sound repeated
 *   ≥3× (converge_count) to lock in a target — random strangers never provide
 *   that. So the body never gets past babbling.
 *
 * What this does:
 *   Spawns a dedicated "nursery body" (its own runtime dir, seeded from master),
 *   connects to its Unix socket as the single client (same role the gateway
 *   plays), and feeds a STRUCTURED, REPEATED acoustic curriculum as 0x01 senses
 *   frames. The body's vocal-imitation loop hears clean, repeated spectral
 *   targets and converges. Periodically the nursery body's learned_bias.bin is
 *   copied to the master so every visitor session inherits the trained voice.
 *
 * Honest scope:
 *   The body's learning clocks are wall-clock (vocal_hz=50), so this is NOT
 *   "faster than realtime" — the win is CONTINUOUS + CORRECT + REPEATED
 *   curriculum instead of intermittent garbage. Foundational phases (tones,
 *   tone-pairs, rhythm, call-and-response) are synthesised analytically as
 *   12-band spectral envelopes — no TTS, no sound card, runs anywhere.
 *
 * Usage:
 *   node nursery_feeder.mjs --body <path> --runtime <dir> [--duration <sec>] [--master <path>]
 *   node nursery_feeder.mjs --body <path> --runtime <dir> --forever --master <path>
 */

import { spawn }            from 'node:child_process'
import { createConnection } from 'node:net'
import {
  existsSync, mkdirSync, copyFileSync, readFileSync, writeFileSync, rmSync, renameSync,
}                           from 'node:fs'
import { dirname, join }    from 'node:path'
import { deserializeCorpus } from './audio_to_bands.mjs'

// ── Frame format (must match net_adapter.rs / security.mjs) ───────────────────
// net_adapter.rs layout (v2):
//   [0x01 tag][level:f32][flux:f32][bands×64:f32][motion:f32][luma:f32]
//   [spectral_centroid:f32][spectral_flatness:f32][spectral_rolloff:f32]
//   [spectral_spread:f32][zcr:f32][hnr:f32][onset_strength:f32][temporal_flux:f32]
//   [grid:160×120×3 f32 planar RGB]
// Total: 1 + (2+64+2+8)×4 + 57600×4 = 230705 bytes
const BANDS        = 64
const AUX_FEATURES = 8          // spectral_centroid…temporal_flux
const GRID_W       = 160
const GRID_H       = 120
const GRID_CH      = 3          // RGB planar
const GRID_PIXELS  = GRID_W * GRID_H          // 19200
const GRID_TOTAL   = GRID_PIXELS * GRID_CH    // 57600
const FRAME_BYTES  = 1 + (2 + BANDS + 2 + AUX_FEATURES) * 4 + GRID_TOTAL * 4  // 230705
const SEND_HZ      = 50
const DT_MS        = 1000 / SEND_HZ

// ── CLI args ──────────────────────────────────────────────────────────────────
const args = process.argv.slice(2)
function arg(name, def = null) {
  const i = args.indexOf(`--${name}`)
  return i !== -1 && i + 1 < args.length ? args[i + 1] : def
}
const BODY_BIN     = arg('body', '../bionic_v0/target/release/body')
// Resolve sandbox_runner NEXT TO the body binary, not relative to the feeder's
// cwd. The old '../sandbox_runner' default resolved (from /home/joe/aborn) to
// /home/joe/sandbox_runner — ENOENT — so the mainline's bytecode VM never ran
// and action_consequence.n_windows was pinned at 0 (the live_io capability gate
// could never pass). dirname(BODY_BIN)/sandbox_runner is correct for both the
// production absolute --body path and the dev target/release default.
const SANDBOX_BIN  = arg('sandbox', join(dirname(BODY_BIN), 'sandbox_runner'))
const RUNTIME_DIR  = arg('runtime', '/tmp/aborn-nursery')
const MASTER_BIAS  = arg('master', null)         // copy learned_bias here periodically
const SEED_DIR     = arg('seed', null)           // dir with learned_bias/vital_core to seed from
const DURATION_S   = parseInt(arg('duration', '0'), 10)  // 0 = forever
const FOREVER      = args.includes('--forever') || DURATION_S === 0
const HARVEST_S    = parseInt(arg('harvest', '120'), 10) // harvest every N sec
// Pool dir the gateway's merge_artifacts ingests from (defaults next to master,
// = the gateway's POOL_DIR). The mainline harvests its PROTOTYPE skills here as
// branch-like contributions so merge_artifacts (the SOLE writer of the master
// prototype files) merges them WITHOUT clobbering visitor contributions.
const POOL_DIR     = arg('pool', (MASTER_BIAS ? join(dirname(MASTER_BIAS), 'training_pool') : null))
const CORPUS_PATH  = arg('corpus', null)         // human-speech corpus_frames.bin → real-audio curriculum
let CORPUS = null                                // loaded utterances [{label, ticks}], or null

function log(...a) {
  console.log(`[nursery ${new Date().toISOString()}] ${a.join(' ')}`)
}

const fmt = (v) => v == null ? 'n/a' : v.toFixed(5)

// ── Frame builder ─────────────────────────────────────────────────────────────
// Compute 8 auxiliary acoustic features from spectral bands.
// These fill the AUX_FEATURES slot that net_adapter.rs reads after luma.
function computeAux(bands, flux) {
  const N = bands.length
  let totalE = 0, wFreq = 0
  for (let i = 0; i < N; i++) { totalE += bands[i]; wFreq += bands[i] * (i / (N - 1)) }
  const centroid = totalE > 1e-8 ? wFreq / totalE : 0.5
  let spread = 0
  for (let i = 0; i < N; i++) {
    const d = i / (N - 1) - centroid
    spread += bands[i] * d * d
  }
  spread = totalE > 1e-8 ? Math.sqrt(spread / totalE) : 0.1
  let logSum = 0, cnt = 0
  for (let i = 0; i < N; i++) { if (bands[i] > 1e-10) { logSum += Math.log(bands[i]); cnt++ } }
  const gm = cnt > 0 ? Math.exp(logSum / cnt) : 0
  const am = totalE / N
  const flatness = am > 1e-10 ? Math.min(gm / am, 1) : 0
  let cum = 0, rolloff = 1.0
  for (let i = 0; i < N; i++) { cum += bands[i]; if (cum >= 0.85 * totalE) { rolloff = i / (N-1); break } }
  return [
    centroid,                       // spectral_centroid
    flatness,                       // spectral_flatness
    rolloff,                        // spectral_rolloff
    spread,                         // spectral_spread
    0.0,                            // zcr  (not meaningful at frame level)
    Math.max(0, 1 - flatness * 2),  // hnr  (1=pure tone, 0=noise)
    Math.max(0, flux),              // onset_strength
    Math.abs(flux),                 // temporal_flux
  ]
}

// Build a silent/uniform frame (used during REST between curriculum passes).
function buildFrame(level, flux, bands, motion, luma, gridValUnused) {
  const buf = Buffer.alloc(FRAME_BYTES)
  let off = 0
  buf.writeUInt8(0x01, off); off += 1
  buf.writeFloatLE(level, off); off += 4
  buf.writeFloatLE(flux,  off); off += 4
  for (let b = 0; b < BANDS; b++) { buf.writeFloatLE(bands[b] ?? 0, off); off += 4 }
  buf.writeFloatLE(motion, off); off += 4
  buf.writeFloatLE(luma,   off); off += 4
  const aux = computeAux(bands, flux)
  for (const a of aux) { buf.writeFloatLE(a, off); off += 4 }
  // Dim uniform grey grid — no spatial structure, just ambient presence.
  for (let g = 0; g < GRID_TOTAL; g++) { buf.writeFloatLE(luma, off); off += 4 }
  return buf
}

// Build a frame with a spatially-structured 160×120 RGB visual field that
// tracks the acoustic level. Fixes the stable_recurrence = 0.006 floor:
//
//   Problem diagnosed: nursery_feeder was sending 3137-byte frames (32×24
//   grayscale, no AUX) while net_adapter.rs expects 230497-byte frames
//   (160×120 RGB + 8 AUX features). The body's read_exact(230497) had to
//   consume ~73 feeder frames to fill one body frame → sensory update rate
//   ~0.68Hz, visual grid entirely garbage. Spatial structure never formed.
//
//   Fix: correct FRAME_BYTES + a centred Gaussian blob (160×120 RGB) that
//   brightens during acoustic events. During silence level=0 → blob vanishes
//   → natural onset/offset creates phot event_energy. Slot persistence while
//   the tone is held drives routing_stability > 0.10 and breaks the floor.
function buildAcousticSyncFrame(tick) {
  const buf = Buffer.alloc(FRAME_BYTES)
  let off = 0
  const luma   = 0.06 + tick.level * 0.10   // 0.06 quiet → 0.16 at peak tone
  const motion = tick.flux * 0.50            // onset/offset transitions
  buf.writeUInt8(0x01, off); off += 1
  buf.writeFloatLE(tick.level, off); off += 4
  buf.writeFloatLE(tick.flux,  off); off += 4
  for (let b = 0; b < BANDS; b++) { buf.writeFloatLE(tick.bands[b] ?? 0, off); off += 4 }
  buf.writeFloatLE(motion, off); off += 4
  buf.writeFloatLE(luma,   off); off += 4
  const aux = computeAux(tick.bands, tick.flux)
  for (const a of aux) { buf.writeFloatLE(a, off); off += 4 }

  // 160×120 RGB planar: [R×19200][G×19200][B×19200]
  // Centred Gaussian blob, sigma ≈ 1/6 of frame. Background 0.04.
  // All 3 channels identical (neutral white blob).
  const cx = GRID_W / 2 - 0.5, cy = GRID_H / 2 - 0.5
  const sx = GRID_W / 6, sy = GRID_H / 6   // sigma ~27px × ~20px
  // Pre-compute pixel values for one channel, then copy to R/G/B.
  const pix = new Float32Array(GRID_PIXELS)
  for (let y = 0; y < GRID_H; y++) {
    for (let x = 0; x < GRID_W; x++) {
      const dx = (x - cx) / sx, dy = (y - cy) / sy
      const blob = Math.exp(-0.5 * (dx * dx + dy * dy))
      pix[y * GRID_W + x] = Math.min(0.04 + blob * tick.level * 0.40, 1.0)
    }
  }
  for (let ch = 0; ch < GRID_CH; ch++) {   // R then G then B
    for (let p = 0; p < GRID_PIXELS; p++) {
      buf.writeFloatLE(pix[p], off); off += 4
    }
  }
  return buf
}

// Gaussian spectral bump centred at c∈[0,1] across the 12 bands. This is what
// the body reduces to a spectral_center in tick_vocal_target_from_vib.
function bandsFromCenter(c, width, energy) {
  const bands = new Array(BANDS).fill(0)
  for (let i = 0; i < BANDS; i++) {
    const f = i / (BANDS - 1)
    const d = (f - c) / width
    bands[i] = energy * Math.exp(-0.5 * d * d)
  }
  return bands
}

// ── Curriculum primitives (generators of per-tick {level,flux,bands}) ─────────
// Energy kept GENTLE (≈0.4): every voicing tick costs 0.5 vital exertion
// regardless of magnitude, so the lever for survival is voicing DUTY CYCLE, not
// loudness. Lessons are short and followed by rests so reserve recovers between
// them (the body voices when fed, rests when it must — like infant-directed
// speech with lots of pauses). Durable learning accumulates in learned_bias
// (seeded across sessions); the within-session prototype count just signals
// that the loop is closing.
function* tone(center, durMs, energy = 0.4, width = 0.18) {
  const n = Math.max(1, Math.round(durMs / DT_MS))
  for (let i = 0; i < n; i++) {
    const env = Math.sin(Math.PI * (i + 0.5) / n)   // smooth attack→decay
    yield { level: energy * env, flux: 0.25 * env, bands: bandsFromCenter(center, width, energy * (0.4 + 0.6 * env)) }
  }
}
function* silence(durMs) {
  const n = Math.max(1, Math.round(durMs / DT_MS))
  for (let i = 0; i < n; i++) yield { level: 0, flux: 0, bands: new Array(BANDS).fill(0) }
}
function* glide(c0, c1, durMs, energy = 0.4) {
  const n = Math.max(1, Math.round(durMs / DT_MS))
  for (let i = 0; i < n; i++) {
    const t = (i + 0.5) / n
    const env = Math.sin(Math.PI * t)
    yield { level: energy * env, flux: 0.3, bands: bandsFromCenter(c0 + (c1 - c0) * t, 0.16, energy * (0.4 + 0.6 * env)) }
  }
}

// Multi-syllable "word" — short tones at distinct spectral centers in sequence.
function* syllableWord(centers, syllableMs = 280, gapMs = 180, energy = 0.32, width = 0.14) {
  for (let i = 0; i < centers.length; i++) {
    yield* tone(centers[i], syllableMs, energy, width)
    if (i < centers.length - 1) yield* silence(gapMs)
  }
}

// One full curriculum pass. Each utterance repeated REPS× so the imitation loop
// can hit converge_count≥3. Phases 1–6: foundational vowels; 7–10: word-like
// targets aligned with training_corpus.json categories. Analytic synthesis only.
const REPS = 3
const REST_MS = 25000   // base quiet window between passes (nursery env is generous)
const RESERVE_LOW = 0.12
const RESERVE_MED = 0.30   // only extend rests when genuinely tired
const VITAL_SEED_RESERVE = 0.55

function passRestMs(getReserve) {
  const reserve = getReserve()
  if (reserve == null) return REST_MS
  if (reserve < RESERVE_LOW) return REST_MS + 25000
  if (reserve < RESERVE_MED) return REST_MS + 10000
  return REST_MS
}

function* adaptiveRest(getReserve, phaseLabel = '') {
  const reserve = getReserve()
  if (reserve == null || reserve >= RESERVE_MED) return
  const extra = reserve < RESERVE_LOW ? 12000 : 6000
  if (phaseLabel) log(`adaptive_rest phase=${phaseLabel} reserve=${reserve.toFixed(4)} extra_ms=${extra}`)
  yield* silence(extra)
}

// Nursery sessions seed from master; bump depleted reserve so training starts fed.
function boostVitalSeed(runtimeDir) {
  const p = join(runtimeDir, 'vital_core.bin')
  if (!existsSync(p)) return
  try {
    const buf = readFileSync(p)
    if (buf.length < 41) return
    const reserve = buf.readDoubleLE(16)
    if (reserve >= VITAL_SEED_RESERVE) return
    const patched = Buffer.from(buf)
    patched.writeDoubleLE(VITAL_SEED_RESERVE, 16)
    patched.writeDoubleLE(Math.min(buf.readDoubleLE(24), 0.05), 24)
    patched[40] = 0 // Alive
    writeFileSync(p, patched)
    log(`boosted seed reserve ${reserve.toFixed(4)} → ${VITAL_SEED_RESERVE}`)
  } catch {}
}

function* curriculumPass(getReserve = () => null) {
  const centers = [0.2, 0.5, 0.8, 0.35, 0.65]   // Phase 1–4: broad vowel targets

  // Phase 1 — single tones (hold a target, repeat)
  for (const c of centers) {
    for (let r = 0; r < REPS; r++) { yield* tone(c, 700); yield* silence(350) }
  }
  yield* adaptiveRest(getReserve, '1')

  // Phase 2 — tone pairs (two targets in sequence, repeat)
  const pairs = [[0.2, 0.6], [0.7, 0.3], [0.4, 0.8]]
  for (const [a, b] of pairs) {
    for (let r = 0; r < REPS; r++) { yield* tone(a, 450); yield* tone(b, 450); yield* silence(400) }
  }
  yield* adaptiveRest(getReserve, '2')

  // Phase 3 — rhythmic pulses (timing, repeat; gentler energy)
  for (let r = 0; r < REPS; r++) {
    for (let k = 0; k < 3; k++) { yield* tone(0.5, 160, 0.32); yield* silence(220) }
    yield* silence(700)
  }
  yield* adaptiveRest(getReserve, '3')

  // Phase 4 — glides / echo (rising then repeat, built-in repetition)
  const glides = [[0.2, 0.8], [0.8, 0.2]]
  for (const [a, b] of glides) {
    for (let r = 0; r < REPS; r++) { yield* glide(a, b, 600); yield* silence(300); yield* glide(a, b, 600); yield* silence(500) }
  }
  yield* adaptiveRest(getReserve, '4')

  // Phase 5 — fine formant targets (narrow bands, new spectral centers)
  const fineCenters = [0.1, 0.15, 0.45, 0.55, 0.85, 0.9]
  const FINE_WIDTH = 0.10
  const FINE_REPS = 2
  for (const c of fineCenters) {
    for (let r = 0; r < FINE_REPS; r++) {
      yield* tone(c, 550, 0.30, FINE_WIDTH)
      yield* silence(500)
    }
  }
  yield* adaptiveRest(getReserve, '5')

  // Phase 6 — burst / silence turn-taking (short triplets, long response window)
  const burstCenters = [0.3, 0.5, 0.7]
  const BURST_REPS = 2
  for (let r = 0; r < BURST_REPS; r++) {
    for (const c of burstCenters) {
      yield* tone(c, 100, 0.28, 0.14)
      yield* silence(280)
    }
    yield* silence(12000)
  }
  yield* adaptiveRest(getReserve, '6')

  // Phase 7 — simple words (2–3 syllable spectral chains, corpus: simple_words)
  const wordPatterns = [
    [0.25, 0.55], [0.15, 0.45, 0.75], [0.35, 0.65],
    [0.20, 0.50], [0.40, 0.70, 0.30], [0.55, 0.25],
    [0.30, 0.60], [0.45, 0.15], [0.65, 0.35, 0.55],
  ]
  const WORD_REPS = 2
  for (const syllables of wordPatterns) {
    for (let r = 0; r < WORD_REPS; r++) {
      yield* syllableWord(syllables, 260, 200, 0.30, 0.13)
      yield* silence(600)
    }
  }
  yield* adaptiveRest(getReserve, '7')

  // Phase 8 — word pairs (two distinct targets, corpus: word_pairs)
  const wordPairs = [
    [[0.20, 0.45], [0.60, 0.80]], [[0.15, 0.35], [0.55, 0.75]],
    [[0.30, 0.50], [0.70, 0.40]], [[0.25, 0.65], [0.45, 0.85]],
    [[0.35, 0.55], [0.75, 0.25]], [[0.40, 0.60], [0.20, 0.80]],
  ]
  const PAIR_REPS = 2
  for (const [a, b] of wordPairs) {
    for (let r = 0; r < PAIR_REPS; r++) {
      yield* syllableWord(a, 240, 180, 0.28, 0.12)
      yield* silence(900)
      yield* syllableWord(b, 240, 180, 0.28, 0.12)
      yield* silence(700)
    }
  }
  yield* adaptiveRest(getReserve, '8')

  // Phase 9 — call and response (question rise + long wait + answer, corpus: call_and_response)
  const callResponse = [
    [0.25, 0.55], [0.30, 0.70], [0.20, 0.60], [0.35, 0.75],
  ]
  const CR_REPS = 2
  for (const [call, answer] of callResponse) {
    for (let r = 0; r < CR_REPS; r++) {
      yield* glide(call, answer, 500, 0.30)
      yield* silence(350)
      yield* tone(answer, 400, 0.26, 0.12)
      yield* silence(10000)
      yield* tone(call, 350, 0.28, 0.14)
      yield* silence(800)
    }
  }
  yield* adaptiveRest(getReserve, '9')

  // Phase 10 — simple sentences (longer chains, corpus: simple_sentences)
  const sentences = [
    [0.20, 0.40, 0.60, 0.45], [0.30, 0.50, 0.70, 0.55],
    [0.15, 0.35, 0.55, 0.75, 0.40], [0.25, 0.45, 0.65],
    [0.35, 0.55, 0.75, 0.50, 0.30],
  ]
  const SENT_REPS = 2
  for (const chain of sentences) {
    for (let r = 0; r < SENT_REPS; r++) {
      yield* syllableWord(chain, 220, 160, 0.27, 0.11)
      yield* silence(1200)
    }
  }
  yield* adaptiveRest(getReserve, '10')

  // Rest between passes — quiet window so vital reserve recovers.
  const restMs = passRestMs(getReserve)
  if (restMs > REST_MS) log(`pass_rest reserve=${fmt(getReserve())} rest_ms=${restMs}`)
  yield* silence(restMs)
}

// ── Master seed / harvest (learned_bias + phoneme memory persist across restarts)
const MASTER_DIR = MASTER_BIAS ? dirname(MASTER_BIAS) : null
const MASTER_ARTIFACTS = [
  ['learned_bias.bin',    MASTER_BIAS],
  ['phoneme_memory.bin',  MASTER_DIR ? join(MASTER_DIR, 'master_phoneme_memory.bin') : null],
  ['familiar_objects.bin', MASTER_DIR ? join(MASTER_DIR, 'master_familiar_objects.bin') : null],
  ['vital_core.bin',      MASTER_DIR ? join(MASTER_DIR, 'master_vital_core.bin') : null],
  ['vocal_memory.bin',    MASTER_DIR ? join(MASTER_DIR, 'master_vocal_memory.bin') : null],
  // Phase 2: the mainline must carry its full vocal + abstraction level.
  // vocal_target_recall (target→correction) and snapshot (cross-layer
  // predictors) were orphaned — harvested/seeded nowhere — so the mainline
  // re-solved them every restart. Include them so the mainline accumulates.
  ['vocal_target_recall.bin', MASTER_DIR ? join(MASTER_DIR, 'master_vocal_target_recall.bin') : null],
  ['snapshot.bin',        MASTER_DIR ? join(MASTER_DIR, 'master_snapshot.bin') : null],
]

// Prototype skills go through the POOL so merge_artifacts (the single writer of
// master_*prototype.bin) MERGES the mainline's + visitors' contributions instead
// of the mainline clobbering them every harvest (the audit's #1: the old
// copyFileSync made the whole branch→pool→mainline pipeline a steady-state no-op).
const POOL_PROTOTYPES = ['vocal_target_recall.bin', 'vocal_memory.bin', 'phoneme_memory.bin']
// These the mainline writes directly to master (not merged by anyone else → no
// clobber). learned_bias is special-cased below to ZERO maturity.
const DIRECT_TO_MASTER = MASTER_ARTIFACTS.filter(([n]) => !POOL_PROTOTYPES.includes(n) && n !== 'learned_bias.bin')

function atomicWrite(dest, buf) {
  const tmp = dest + '.tmp'
  writeFileSync(tmp, buf)
  renameSync(tmp, dest)
}

function harvestMasterArtifacts(runtimeDir) {
  // 1) Prototype skills → pool as a mainline contribution (merge_artifacts merges).
  if (POOL_DIR) {
    try { mkdirSync(POOL_DIR, { recursive: true }) } catch {}
    const stamp = `${Date.now()}_mainline`
    for (const name of POOL_PROTOTYPES) {
      const src = join(runtimeDir, name)
      if (!existsSync(src)) continue
      try { atomicWrite(join(POOL_DIR, `${stamp}__${name}`), readFileSync(src)) } catch {}
    }
  }
  // 2) learned_bias → master, but ZERO the trailing maturity f64. Maturity is a
  //    per-life developmental clock, not a shared skill; transporting the
  //    mainline's saturated 1.0 made every visitor seed "fake-mature" (low
  //    plasticity + the mat≈0.9999 illusion). Visitors now start their own clock.
  if (MASTER_BIAS) {
    const src = join(runtimeDir, 'learned_bias.bin')
    if (existsSync(src)) {
      try {
        const buf = Buffer.from(readFileSync(src))
        if (buf.length >= 8) buf.writeDoubleLE(0, buf.length - 8)  // maturity := 0
        atomicWrite(MASTER_BIAS, buf)
      } catch {}
    }
  }
  // 3) Relationship / physiological / cross-layer state → direct (no merge conflict).
  for (const [name, dest] of DIRECT_TO_MASTER) {
    if (!dest) continue
    const src = join(runtimeDir, name)
    if (!existsSync(src)) continue
    try { atomicWrite(dest, readFileSync(src)) } catch {}
  }
}

function seedFromMaster(runtimeDir) {
  for (const [name, src] of MASTER_ARTIFACTS) {
    if (!src || !existsSync(src)) continue
    try { copyFileSync(src, join(runtimeDir, name)) } catch {}
  }
}

// ── learned_bias.bin v5 tail = [..., prediction_familiarity, action_suppression,
// vocal_efficacy, developmental_maturity] (4 trailing f64). Under v4 the last
// f64 was vocal_efficacy; v5 appended maturity, so efficacy is now at -16.
function readVocalEfficacy(path) {
  if (!existsSync(path)) return null
  try {
    const buf = readFileSync(path)
    return buf.length >= 16 ? buf.readDoubleLE(buf.length - 16) : null
  } catch { return null }
}
function readMaturity(path) {
  if (!existsSync(path)) return null
  try {
    const buf = readFileSync(path)
    return buf.length >= 8 ? buf.readDoubleLE(buf.length - 8) : null
  } catch { return null }
}
function readReserve(runtimeDir) {
  const p = join(runtimeDir, 'vital_core.bin')
  if (!existsSync(p)) return null
  try {
    const buf = readFileSync(p)
    return buf.length >= 24 ? buf.readDoubleLE(16) : null
  } catch { return null }
}

// ── Main ──────────────────────────────────────────────────────────────────────
const socketPath = join(RUNTIME_DIR, 'net.sock')
const biasPath   = join(RUNTIME_DIR, 'learned_bias.bin')

// Fresh runtime dir; seed learned_bias + personal files so it starts trained & alive.
rmSync(RUNTIME_DIR, { recursive: true, force: true })
mkdirSync(RUNTIME_DIR, { recursive: true })
if (SEED_DIR) {
  for (const f of ['learned_bias.bin', 'vital_core.bin', 'familiar_objects.bin', 'self_model.bin', 'phoneme_memory.bin', 'vocal_memory.bin', 'vocal_target_recall.bin', 'snapshot.bin']) {
    const src = join(SEED_DIR, f)
    if (existsSync(src)) { try { copyFileSync(src, join(RUNTIME_DIR, f)) } catch {} }
  }
} else {
  seedFromMaster(RUNTIME_DIR)
}
boostVitalSeed(RUNTIME_DIR)

// sandbox_runner reads live_bytes/out.frames (body's bytecode output), executes
// it in a stack VM, and writes results back to live_bytes/in.frames. Without it
// running, action_consequence.n_windows stays at 0 forever — the body never
// forms return windows and the live_io gate can never pass.
mkdirSync(join(RUNTIME_DIR, 'live_bytes'), { recursive: true })
log(`spawning sandbox_runner: ${SANDBOX_BIN} ${RUNTIME_DIR}`)
const sandbox = spawn(SANDBOX_BIN, [RUNTIME_DIR], { stdio: 'ignore' })
sandbox.on('error', (e) => log(`sandbox_runner spawn error (non-fatal): ${e.message}`))
sandbox.on('exit', (code) => log(`sandbox_runner exited code=${code}`))

log(`spawning body: ${BODY_BIN} ${RUNTIME_DIR} --nursery-bias --headless ${socketPath}`)
const child = spawn(BODY_BIN, [RUNTIME_DIR, '--nursery-bias', '--headless', socketPath], { stdio: 'ignore' })
child.on('exit', (code) => { log(`body exited code=${code}`); sandbox.kill('SIGTERM'); process.exit(code ?? 0) })

const startEff = readVocalEfficacy(biasPath)
log(`seed vocal_efficacy=${startEff ?? 'n/a'} maturity=${readMaturity(biasPath) ?? 'n/a'} reserve=${readReserve(RUNTIME_DIR) ?? 'n/a'}`)
loadCorpus()

let sock = null
let stopped = false
const startMs = Date.now()

function connect(attempt = 0) {
  if (attempt > 40) { log('body socket never appeared — aborting'); child.kill('SIGTERM'); process.exit(1) }
  if (!existsSync(socketPath)) { setTimeout(() => connect(attempt + 1), 250); return }
  const c = createConnection(socketPath)
  c.on('error', () => setTimeout(() => connect(attempt + 1), 250))
  c.once('connect', () => { sock = c; log('connected to body socket — feeding curriculum'); feedLoop() })
  c.on('data', () => {})  // drain vocal/surface output the body sends back
  c.on('close', () => { if (!stopped) { log('socket closed'); } })
}

// ── Real-audio curriculum (human speech corpus → identical per-tick frames) ───
function loadCorpus() {
  if (!CORPUS_PATH) return
  if (!existsSync(CORPUS_PATH)) {
    log(`corpus path not found: ${CORPUS_PATH} (real-audio curriculum disabled)`); return
  }
  try {
    CORPUS = deserializeCorpus(readFileSync(CORPUS_PATH))
    if (CORPUS && CORPUS.length) {
      const totalTicks = CORPUS.reduce((a, u) => a + u.ticks.length, 0)
      log(`real-audio corpus: ${CORPUS.length} utterances, ${totalTicks} ticks [${CORPUS.map(u => u.label).join(' ')}]`)
    } else { CORPUS = null; log('corpus deserialize failed or empty — disabled') }
  } catch (e) { CORPUS = null; log(`corpus load error: ${e.message} — disabled`) }
}

// Replay each real utterance REPS× with rests — same infant-directed pacing as
// the synthetic phases. Corpus ticks are already at DT_MS cadence (offline xform),
// so one tick per feedLoop iteration plays back in real time.
function* realAudioPass(getReserve) {
  if (!CORPUS) return
  for (const utt of CORPUS) {
    for (let r = 0; r < REPS; r++) {
      for (const tk of utt.ticks) yield tk
      yield* silence(350)
    }
    yield* adaptiveRest(getReserve, `real:${utt.label}`)
  }
}

// One full cycle: synthetic foundational phases, then real human speech.
function* fullPass(getReserve) {
  yield* curriculumPass(getReserve)
  yield* realAudioPass(getReserve)
}

async function feedLoop() {
  let lastHarvest = Date.now()
  let lastReport  = Date.now()
  while (!stopped) {
    for (const tick of fullPass(() => readReserve(RUNTIME_DIR))) {
      if (stopped || !sock || sock.destroyed) return
      const frame = buildAcousticSyncFrame(tick)
      sock.write(frame)
      await sleep(DT_MS)

      const now = Date.now()
      if (now - lastReport >= 30_000) {
        lastReport = now
        log(`t=${Math.round((now - startMs) / 1000)}s vocal_efficacy=${fmt(readVocalEfficacy(biasPath))} maturity=${fmt(readMaturity(biasPath))} reserve=${fmt(readReserve(RUNTIME_DIR))}`)
      }
      if (MASTER_BIAS && now - lastHarvest >= HARVEST_S * 1000) {
        lastHarvest = now
        if (existsSync(biasPath)) {
          try {
            harvestMasterArtifacts(RUNTIME_DIR)
            log('harvested runtime artifacts → master')
          } catch {}
        }
      }
      if (!FOREVER && now - startMs >= DURATION_S * 1000) { finish(); return }
    }
  }
}

function finish() {
  stopped = true
  const endEff = readVocalEfficacy(biasPath)
  log(`DONE start_eff=${fmt(startEff)} end_eff=${fmt(endEff)} delta=${fmt((endEff ?? 0) - (startEff ?? 0))}`)
  if (MASTER_BIAS && existsSync(biasPath)) {
    try { harvestMasterArtifacts(RUNTIME_DIR); log('final harvest → master') } catch {}
  }
  sock?.end()
  setTimeout(() => { sandbox.kill('SIGTERM'); child.kill('SIGTERM'); process.exit(0) }, 500)
}

const sleep = (ms) => new Promise(r => setTimeout(r, ms))

process.on('SIGINT',  finish)
process.on('SIGTERM', finish)

connect()
