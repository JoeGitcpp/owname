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
  existsSync, mkdirSync, copyFileSync, readFileSync, rmSync,
}                           from 'node:fs'
import { join }             from 'node:path'

// ── Frame format (must match net_adapter.rs / security.mjs) ───────────────────
const BANDS       = 12
const GRID        = 32 * 24
const FRAME_BYTES = 1 + (2 + BANDS + 2) * 4 + GRID * 4   // 3137
const SEND_HZ     = 50
const DT_MS       = 1000 / SEND_HZ

// ── CLI args ──────────────────────────────────────────────────────────────────
const args = process.argv.slice(2)
function arg(name, def = null) {
  const i = args.indexOf(`--${name}`)
  return i !== -1 && i + 1 < args.length ? args[i + 1] : def
}
const BODY_BIN     = arg('body', '../bionic_v0/target/release/body')
const RUNTIME_DIR  = arg('runtime', '/tmp/aborn-nursery')
const MASTER_BIAS  = arg('master', null)         // copy learned_bias here periodically
const SEED_DIR     = arg('seed', null)           // dir with learned_bias/vital_core to seed from
const DURATION_S   = parseInt(arg('duration', '0'), 10)  // 0 = forever
const FOREVER      = args.includes('--forever') || DURATION_S === 0
const HARVEST_S    = parseInt(arg('harvest', '120'), 10) // copy→master every N sec

function log(...a) {
  console.log(`[nursery ${new Date().toISOString()}] ${a.join(' ')}`)
}

// ── Frame builder ─────────────────────────────────────────────────────────────
function buildFrame(level, flux, bands, motion, luma, gridVal) {
  const buf = Buffer.alloc(FRAME_BYTES)
  let off = 0
  buf.writeUInt8(0x01, off); off += 1
  buf.writeFloatLE(level, off); off += 4
  buf.writeFloatLE(flux,  off); off += 4
  for (let b = 0; b < BANDS; b++) { buf.writeFloatLE(bands[b] ?? 0, off); off += 4 }
  buf.writeFloatLE(motion, off); off += 4
  buf.writeFloatLE(luma,   off); off += 4
  // Dim ambient visual presence keeps the body's vital state alive without motion.
  for (let g = 0; g < GRID; g++) { buf.writeFloatLE(gridVal, off); off += 4 }
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

// One full curriculum pass. Each utterance repeated REPS× so the imitation loop
// can hit converge_count≥3. Foundational acoustic phases only — analytic, no TTS.
const REPS = 4
const REST_MS = 15000   // quiet window between passes so reserve recovers
function* curriculumPass() {
  const centers = [0.2, 0.5, 0.8, 0.35, 0.65]   // "vowel" spectral targets

  // Phase 1 — single tones (hold a target, repeat)
  for (const c of centers) {
    for (let r = 0; r < REPS; r++) { yield* tone(c, 700); yield* silence(350) }
  }
  // Phase 2 — tone pairs (two targets in sequence, repeat)
  const pairs = [[0.2, 0.6], [0.7, 0.3], [0.4, 0.8]]
  for (const [a, b] of pairs) {
    for (let r = 0; r < REPS; r++) { yield* tone(a, 450); yield* tone(b, 450); yield* silence(400) }
  }
  // Phase 3 — rhythmic pulses (timing, repeat)
  for (let r = 0; r < REPS; r++) {
    for (let k = 0; k < 4; k++) { yield* tone(0.5, 180, 0.5); yield* silence(180) }
    yield* silence(500)
  }
  // Phase 4 — glides / call-and-response (rising then echo, built-in repetition)
  const glides = [[0.2, 0.8], [0.8, 0.2]]
  for (const [a, b] of glides) {
    for (let r = 0; r < REPS; r++) { yield* glide(a, b, 600); yield* silence(300); yield* glide(a, b, 600); yield* silence(500) }
  }

  // Rest between passes — a long quiet window so the body stops voicing and
  // vital reserve recovers (voicing costs 0.5 drain/tick; only quiet lets
  // drain fall back to the 0.2 base so supply can lift reserve again).
  yield* silence(REST_MS)
}

// ── vocal_efficacy = last 8 bytes of learned_bias.bin (final write_f64) ───────
function readVocalEfficacy(path) {
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
  for (const f of ['learned_bias.bin', 'vital_core.bin', 'familiar_objects.bin', 'self_model.bin']) {
    const src = join(SEED_DIR, f)
    if (existsSync(src)) { try { copyFileSync(src, join(RUNTIME_DIR, f)) } catch {} }
  }
} else if (MASTER_BIAS && existsSync(MASTER_BIAS)) {
  try { copyFileSync(MASTER_BIAS, biasPath) } catch {}
}

log(`spawning body: ${BODY_BIN} ${RUNTIME_DIR} --headless ${socketPath}`)
const child = spawn(BODY_BIN, [RUNTIME_DIR, '--headless', socketPath], { stdio: 'ignore' })
child.on('exit', (code) => { log(`body exited code=${code}`); process.exit(code ?? 0) })

const startEff = readVocalEfficacy(biasPath)
log(`seed vocal_efficacy=${startEff ?? 'n/a'} reserve=${readReserve(RUNTIME_DIR) ?? 'n/a'}`)

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

async function feedLoop() {
  let lastHarvest = Date.now()
  let lastReport  = Date.now()
  while (!stopped) {
    for (const tick of curriculumPass()) {
      if (stopped || !sock || sock.destroyed) return
      const frame = buildFrame(tick.level, tick.flux, tick.bands, 0, 0.08, 0.08)
      sock.write(frame)
      await sleep(DT_MS)

      const now = Date.now()
      if (now - lastReport >= 30_000) {
        lastReport = now
        log(`t=${Math.round((now - startMs) / 1000)}s vocal_efficacy=${fmt(readVocalEfficacy(biasPath))} reserve=${fmt(readReserve(RUNTIME_DIR))}`)
      }
      if (MASTER_BIAS && now - lastHarvest >= HARVEST_S * 1000) {
        lastHarvest = now
        if (existsSync(biasPath)) { try { copyFileSync(biasPath, MASTER_BIAS); log(`harvested learned_bias → master`) } catch {} }
      }
      if (!FOREVER && now - startMs >= DURATION_S * 1000) { finish(); return }
    }
  }
}

function finish() {
  stopped = true
  const endEff = readVocalEfficacy(biasPath)
  log(`DONE start_eff=${fmt(startEff)} end_eff=${fmt(endEff)} delta=${fmt((endEff ?? 0) - (startEff ?? 0))}`)
  if (MASTER_BIAS && existsSync(biasPath)) { try { copyFileSync(biasPath, MASTER_BIAS); log('final harvest → master') } catch {} }
  sock?.end()
  setTimeout(() => { child.kill('SIGTERM'); process.exit(0) }, 500)
}

const sleep = (ms) => new Promise(r => setTimeout(r, ms))
const fmt = (v) => v == null ? 'n/a' : v.toFixed(5)

process.on('SIGINT',  finish)
process.on('SIGTERM', finish)

connect()
