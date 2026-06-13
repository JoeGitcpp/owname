/**
 * training_pool.mjs — distributed training result collection and merging.
 *
 * When a session ends, evaluate_session() grades the learned_bias.bin in the
 * session's runtime dir. Sessions that pass the quality gate are copied into
 * the training pool. Periodically, merge_pool() combines all pool entries into
 * a master learned_bias.bin which new sessions inherit.
 *
 * Evaluation criteria (reject noise / idle sessions):
 *   - session_secs >= MIN_SESSION_SECS     (ran long enough)
 *   - top_recognition_count >= MIN_RECOG   (real sensory input was processed)
 *   - body was not stuck in Standby entire time
 *   - learned_bias actually changed (any delta above DELTA_THRESHOLD)
 *   - vocal_efficacy showed growth (body had real closure events)
 *
 * Merge strategy: EMA — each accepted session's bias nudges the master by
 * MERGE_ALPHA. The master drifts toward the centroid of good sessions without
 * being dominated by any single one. Only axes with significant change are
 * merged (noise-gate per axis).
 */

import { readFileSync, writeFileSync, copyFileSync, existsSync, mkdirSync, readdirSync, unlinkSync } from 'node:fs'
import { join } from 'node:path'

// ── Evaluation thresholds ─────────────────────────────────────────────────────
const MIN_SESSION_SECS  = 5 * 60          // 5 minutes minimum
const MIN_RECOG         = 20              // object recognition events
const DELTA_THRESHOLD   = 0.005          // minimum meaningful bias change
const VOCAL_EFF_MIN     = 0.01           // vocal_efficacy must have grown past default
const MERGE_ALPHA       = 0.05           // how strongly each session nudges master

// ── Relaxed gate (ABORN_HARVEST_RELAXED=1) ───────────────────────────────────
// Short sessions (60–300s) qualify on bias delta alone — skips recognition and
// vocal-efficacy checks.  Intended for early deployments where sessions rarely
// reach 5 min but are still producing real learning signal.
const RELAXED_GATE      = process.env.ABORN_HARVEST_RELAXED === '1'
const RELAXED_MIN_SECS  = 60             // 1-minute floor even in relaxed mode

// ── Learned-bias binary layout (v4) ──────────────────────────────────────────
// Offsets validated against learned_bias.rs
const MAGIC_OFFSET         = 0   // u64 (8 bytes)
const VERSION_OFFSET       = 8   // u64 (8 bytes)
const CURVE_COUNT_OFFSET   = 16  // u32 (4 bytes)
const VOCAL_EFF_OFFSET     = 20  // f64 (8 bytes) — after curve_count in v4
// (exact layout depends on learned_bias serialization; we read conservatively)

/**
 * Grade one session. Returns { pass: bool, score: number, reason: string }.
 *
 * Reads vital_core.bin and familiar_objects.bin for life signals, and
 * learned_bias.bin for delta evidence.
 */
export function evaluate_session({
  runtimeDir,
  startedAt = 0,
  endedAt   = Date.now(),
}) {
  const sessionSecs = (endedAt - startedAt) / 1000

  // ── Gate 0 (relaxed mode): short sessions qualify on bias delta alone ──────
  // Only active when ABORN_HARVEST_RELAXED=1.  Sessions ≥ MIN_SESSION_SECS
  // always use the full strict path below, regardless of this flag.
  if (RELAXED_GATE && sessionSecs >= RELAXED_MIN_SECS && sessionSecs < MIN_SESSION_SECS) {
    try {
      const lb  = readFileSync(join(runtimeDir, 'learned_bias.bin'))
      const dv  = new DataView(lb.buffer, lb.byteOffset, lb.byteLength)
      for (let off = 28; off + 8 <= lb.length; off += 8) {
        const v = Math.abs(dv.getFloat64(off, true))
        if (v > DELTA_THRESHOLD && v < 1e6) {
          return { pass: true, score: 0.3, reason: 'delta_relaxed' }
        }
      }
      return { pass: false, score: 0, reason: 'no_delta_relaxed' }
    } catch {
      return { pass: false, score: 0, reason: 'no_bias_file' }
    }
  }

  // ── Gate 1: duration ───────────────────────────────────────────────────────
  if (sessionSecs < MIN_SESSION_SECS) {
    return { pass: false, score: 0, reason: `too_short:${Math.round(sessionSecs)}s` }
  }

  // ── Gate 2: recognition count (familiar_objects.bin) ──────────────────────
  let topRecog = 0
  let topFam   = 0
  try {
    const fo = readFileSync(join(runtimeDir, 'familiar_objects.bin'))
    if (fo.length >= 20) {
      const dv  = new DataView(fo.buffer, fo.byteOffset, fo.byteLength)
      const n   = dv.getUint32(16, true)      // n_objects at offset 16
      let off   = 20
      for (let i = 0; i < n && off + 52 <= fo.length; i++) {
        off += 32  // fingerprint 4×f64
        const fam = dv.getFloat64(off, true); off += 8
        const rec = dv.getUint32(off, true);  off += 4
        off += 8   // last_seen_us
        if (fam > topFam) { topFam = fam; topRecog = rec }
      }
    }
  } catch { /* no familiar_objects yet — fail gate */ }

  if (topRecog < MIN_RECOG) {
    return { pass: false, score: 0, reason: `low_recog:${topRecog}` }
  }

  // ── Gate 3: vital state (vital_core.bin) ──────────────────────────────────
  let everAlive    = false
  let finalReserve = 0
  try {
    const vc = readFileSync(join(runtimeDir, 'vital_core.bin'))
    if (vc.length >= 41) {
      const dv   = new DataView(vc.buffer, vc.byteOffset, vc.byteLength)
      finalReserve = dv.getFloat64(16, true)
      const vstate = vc[40]                    // 0=Alive, 1=Standby
      // If final reserve > 0 the body lived at some point
      everAlive = finalReserve > 0.05 || vstate === 0
    }
  } catch { }

  if (!everAlive) {
    return { pass: false, score: 0, reason: 'standby_only' }
  }

  // ── Gate 4 + 5: learned_bias delta + vocal_efficacy ──────────────────────
  let hasDelta      = false
  let vocalEfficacy = 0
  try {
    const lb  = readFileSync(join(runtimeDir, 'learned_bias.bin'))
    const dv  = new DataView(lb.buffer, lb.byteOffset, lb.byteLength)
    // Read vocal_efficacy (at known offset in v4 format)
    // Structure: magic(8)+version(8)+n_curves(4)+vocal_efficacy(8)+...
    if (lb.length >= 28) {
      vocalEfficacy = dv.getFloat64(20, true)
    }
    // Scan for any non-trivial f64 delta in the bias data after the header
    for (let off = 28; off + 8 <= lb.length; off += 8) {
      const v = Math.abs(dv.getFloat64(off, true))
      if (v > DELTA_THRESHOLD && v < 1e6) { hasDelta = true; break }
    }
  } catch { }

  if (!hasDelta) {
    return { pass: false, score: 0, reason: 'no_delta' }
  }
  if (vocalEfficacy < VOCAL_EFF_MIN) {
    return { pass: false, score: 0, reason: `low_vocal_eff:${vocalEfficacy.toFixed(4)}` }
  }

  // ── Score (0–1) for sorting/weighting ─────────────────────────────────────
  const durationScore = Math.min(1, sessionSecs / (30 * 60))   // saturates at 30 min
  const recogScore    = Math.min(1, topRecog / 500)
  const famScore      = topFam
  const vocalScore    = Math.min(1, vocalEfficacy * 10)
  const score = (durationScore * 0.3 + recogScore * 0.3 + famScore * 0.2 + vocalScore * 0.2)

  return { pass: true, score, reason: 'ok' }
}

/**
 * Collect a passing session's learned_bias.bin into the pool.
 * Pool files are named by timestamp to avoid collisions.
 */
export function collect_to_pool(runtimeDir, poolDir, sessionId) {
  const src = join(runtimeDir, 'learned_bias.bin')
  if (!existsSync(src)) return false
  mkdirSync(poolDir, { recursive: true })
  const dst = join(poolDir, `${Date.now()}_${sessionId}.bin`)
  copyFileSync(src, dst)
  return dst
}

/**
 * Merge all pool files into the master learned_bias.bin using EMA.
 * Only axes with significant delta are merged (noise gate).
 * After merging, pool files older than MAX_POOL_AGE_DAYS are pruned.
 */
export function merge_pool(poolDir, masterPath, { maxPoolAgeDays = 7 } = {}) {
  if (!existsSync(poolDir)) return { merged: 0, skipped: 0 }

  const poolFiles = readdirSync(poolDir)
    // learned_bias pool files only ({ts}_{sid}.bin). Exclude the prototype
    // artifacts ({ts}_..__<name>.bin) — those are merged by merge_artifacts, not
    // EMA'd as learned_bias (a length guard already skipped them, but be explicit).
    .filter(f => f.endsWith('.bin') && !f.includes('__'))
    .map(f => join(poolDir, f))

  if (poolFiles.length === 0) return { merged: 0, skipped: 0 }

  // Load master (or start from first pool file if no master yet)
  let master = existsSync(masterPath)
    ? Buffer.from(readFileSync(masterPath))
    : Buffer.from(readFileSync(poolFiles[0]))

  let merged = 0
  let skipped = 0

  for (const pf of poolFiles) {
    try {
      const poolBuf = readFileSync(pf)
      if (poolBuf.length !== master.length) { skipped++; continue }

      // EMA merge: master[i] = (1-alpha)*master[i] + alpha*pool[i]
      // Only for f64 values after the header (offset 28+), and only if
      // the pool value differs significantly from master (noise gate).
      const masterDV = new DataView(master.buffer, master.byteOffset, master.byteLength)
      const poolDV   = new DataView(poolBuf.buffer, poolBuf.byteOffset, poolBuf.byteLength)
      let changed = false

      // NOTE: stop 8 bytes before EOF — the trailing f64 is developmental_maturity,
      // a monotonic per-life CLOCK, not a skill axis. EMA-merging it (audit #3) let
      // a high-maturity branch ratchet master maturity regardless of competence
      // (the "maturity 1.0 / efficacy 0.24" decoupling). Maturity must be derived,
      // not transported; leave master's maturity untouched here.
      for (let off = 28; off + 8 <= master.length - 8; off += 8) {
        const mv = masterDV.getFloat64(off, true)
        const pv = poolDV.getFloat64(off, true)
        if (!isFinite(mv) || !isFinite(pv)) continue
        if (Math.abs(pv - mv) < DELTA_THRESHOLD) continue  // noise gate
        const merged_v = mv * (1 - MERGE_ALPHA) + pv * MERGE_ALPHA
        masterDV.setFloat64(off, merged_v, true)
        changed = true
      }

      if (changed) merged++
    } catch {
      skipped++
    }
  }

  if (merged > 0) {
    writeFileSync(masterPath, master)
  }

  // Prune old pool files
  const cutoff = Date.now() - maxPoolAgeDays * 86_400_000
  for (const pf of poolFiles) {
    const ts = parseInt(pf.split('/').pop()?.split('_')[0] || '0', 10)
    if (ts < cutoff) { try { unlinkSync(pf) } catch { } }
  }

  return { merged, skipped }
}
