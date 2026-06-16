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

import { readFileSync, writeFileSync, copyFileSync, existsSync, mkdirSync, readdirSync, unlinkSync, renameSync } from 'node:fs'
import { join } from 'node:path'

// ── Evaluation thresholds ─────────────────────────────────────────────────────
const MIN_SESSION_SECS  = 5 * 60          // 5 minutes minimum
const MIN_RECOG         = 20              // object recognition events
const DELTA_THRESHOLD   = 0.005          // minimum meaningful bias change
const VOCAL_EFF_MIN     = 0.01           // vocal_efficacy must have grown past default
const MERGE_ALPHA       = 0.05           // max nudge (at score 1); scaled by session score
// Merge noise gate, DECOUPLED from the admission DELTA_THRESHOLD. Admission asks
// "did this session learn anything real?" (a coarse 0.005). The merge asks, per
// axis, "is this axis's move above float noise?" — a finer bar, so a good session
// moves more of the master instead of being gated out by the admission threshold.
const MERGE_NOISE_GATE  = 0.0005

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

// ── Familiar-objects checkpoint binary layout ───────────────────────────────
// Mirrors bionic_v0/src/familiar_objects.rs::write_familiar_objects_checkpoint:
//   header : MAGIC u64 + VERSION u64 + count u32                       = 20 bytes
//   object : fingerprint OBJECT_FINGERPRINT_DIM(8)×f64 + familiarity f64
//            + recognition_count u32 + last_seen_us u64                = 84 bytes
//   trailer: last_update_t_us u64                                      =  8 bytes
// NOTE: the fingerprint is 8 f64 (VIB half [0..4] + PHOT half [4..8]), NOT 4.
// A prior 52-byte stride (4-f64 fingerprint) misread recognition_count as
// garbage, silently corrupting Gate 2 below.
const FAM_HEADER_BYTES      = 20
const FAM_COUNT_OFFSET      = 16
const FAM_FINGERPRINT_BYTES = 64   // 8 × f64
const FAM_OBJ_STRIDE        = FAM_FINGERPRINT_BYTES + 8 + 4 + 8   // = 84

/**
 * Parse a familiar_objects.bin buffer → { count, topFam, topRecog, objects }.
 * topRecog is the recognition_count of the MOST-FAMILIAR object (highest familiarity).
 */
export function parseFamiliarObjects(buf) {
  const empty = { count: 0, topFam: 0, topRecog: 0, objects: [] }
  if (!buf || buf.length < FAM_HEADER_BYTES) return empty
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength)
  const count = dv.getUint32(FAM_COUNT_OFFSET, true)
  const objects = []
  let topFam = 0, topRecog = 0
  let off = FAM_HEADER_BYTES
  for (let i = 0; i < count && off + FAM_OBJ_STRIDE <= buf.length; i++) {
    const fam = dv.getFloat64(off + FAM_FINGERPRINT_BYTES, true)
    const rec = dv.getUint32(off + FAM_FINGERPRINT_BYTES + 8, true)
    objects.push({ familiarity: fam, recognitionCount: rec })
    if (fam > topFam) { topFam = fam; topRecog = rec }
    off += FAM_OBJ_STRIDE
  }
  return { count, topFam, topRecog, objects }
}

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
    const parsed = parseFamiliarObjects(readFileSync(join(runtimeDir, 'familiar_objects.bin')))
    topFam   = parsed.topFam
    topRecog = parsed.topRecog
  } catch { /* no familiar_objects yet — fail gate */ }

  if (topRecog < MIN_RECOG) {
    return { pass: false, score: 0, reason: `low_recog:${topRecog}` }
  }

  // ── Gate 3: vital state (vital_core.bin) ──────────────────────────────────
  // We only have the TEARDOWN checkpoint; its reserve/vstate are the FINAL
  // instant. A session that engaged for minutes then dipped to Standby (a talky
  // call drained by the body's voicing_cost + the simulated famine schedule)
  // would look "standby_only" here and be wrongly discarded — the more engaged
  // the call, the more likely it powers down. Sustained recognition is the
  // trustworthy liveness signal: Gate 2 above already proved topRecog ≥
  // MIN_RECOG recognitions, which only accrue while perceiving. So an engaged
  // session counts as having been alive even if it ends tired; a truly parked
  // session is already rejected by Gate 2 (low_recog).
  let aliveAtEnd   = false
  let finalReserve = 0
  try {
    const vc = readFileSync(join(runtimeDir, 'vital_core.bin'))
    if (vc.length >= 41) {
      const dv   = new DataView(vc.buffer, vc.byteOffset, vc.byteLength)
      finalReserve = dv.getFloat64(16, true)
      const vstate = vc[40]                    // 0=Alive, 1=Standby
      aliveAtEnd = finalReserve > 0.05 || vstate === 0
    }
  } catch { }

  if (!aliveAtEnd && topRecog < MIN_RECOG) {
    return { pass: false, score: 0, reason: 'standby_only' }
  }

  // ── Gate 4 + 5: learned_bias delta + vocal_efficacy ──────────────────────
  let hasDelta      = false
  let vocalEfficacy = 0
  try {
    const lb  = readFileSync(join(runtimeDir, 'learned_bias.bin'))
    const dv  = new DataView(lb.buffer, lb.byteOffset, lb.byteLength)
    // Read vocal_efficacy from the learned_bias v5 TAIL. The trailing f64s are
    // [..., vocal_efficacy, developmental_maturity], so efficacy is at length-16
    // (maturity at length-8) — matching nursery_feeder.readVocalEfficacy. The old
    // fixed offset 20 was a stale v4 layout and misread efficacy as ~0, which
    // wrongly rejected genuinely-engaged calls as `low_vocal_eff` (the dashboard
    // exposed exactly this: a 6-min call with closures_with_evidence=1466 and a
    // real efficacy of ~0.13–0.24 in vocal_diag, read here as 0.0000).
    if (lb.length >= 16) {
      vocalEfficacy = dv.getFloat64(lb.length - 16, true)
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
export function collect_to_pool(runtimeDir, poolDir, sessionId, score = 1) {
  const src = join(runtimeDir, 'learned_bias.bin')
  if (!existsSync(src)) return false
  mkdirSync(poolDir, { recursive: true })
  // Encode the quality score (0–1) as `_s<0..1000>` so merge_pool can weight a
  // rich session above a marginal one — the score was computed by evaluate_session
  // then thrown away, so every accepted session nudged the master equally. Single
  // underscore keeps the file out of merge_artifacts' `__<name>` prototype glob,
  // and ts is still split('_')[0] for pruning. Clamped+integer so it always parses.
  const s = Math.max(0, Math.min(1000, Math.round((Number.isFinite(score) ? score : 1) * 1000)))
  // Atomic: copy to .tmp then rename, so the periodic merge never reads a
  // half-written pool file (audit theme 5).
  const dst = join(poolDir, `${Date.now()}_${sessionId}_s${s}.bin`)
  copyFileSync(src, dst + '.tmp')
  renameSync(dst + '.tmp', dst)
  return dst
}

// Parse the `_s<int>` score tag from a pool filename → weight in [0,1].
// Files without the tag (legacy names) default to 1 (neutral, full weight).
function poolScoreOf(filename) {
  const m = /_s(\d+)\.bin$/.exec(filename)
  if (!m) return 1
  return Math.max(0, Math.min(1, parseInt(m[1], 10) / 1000))
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
  let axesMoved = 0

  for (const pf of poolFiles) {
    try {
      const poolBuf = readFileSync(pf)
      if (poolBuf.length !== master.length) { skipped++; continue }

      // Score-weighted EMA: a rich session (score≈1) nudges the master at the full
      // MERGE_ALPHA; a marginal one (e.g. relaxed-gate score 0.3) nudges ~⅓ as hard,
      // instead of equally. Zero-score files contribute nothing.
      const alpha = MERGE_ALPHA * poolScoreOf(pf.split('/').pop() || '')
      if (alpha <= 0) { skipped++; continue }

      // EMA merge: master[i] = (1-alpha)*master[i] + alpha*pool[i]
      // Only for f64 values after the header (offset 28+), and only if the pool
      // value differs from master by more than float noise (MERGE_NOISE_GATE,
      // decoupled from the coarser admission DELTA_THRESHOLD).
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
        if (Math.abs(pv - mv) < MERGE_NOISE_GATE) continue  // float-noise gate
        const merged_v = mv * (1 - alpha) + pv * alpha
        masterDV.setFloat64(off, merged_v, true)
        changed = true
        axesMoved++
      }

      if (changed) merged++
    } catch {
      skipped++
    }
  }

  if (merged > 0) {
    writeFileSync(masterPath, master)
  }
  // Visibility: how much the master actually moved this cycle (silent before).
  console.log(`[training_pool] merge_pool: merged=${merged} skipped=${skipped} axes_moved=${axesMoved} pool=${poolFiles.length}`)

  // Prune old pool files
  const cutoff = Date.now() - maxPoolAgeDays * 86_400_000
  for (const pf of poolFiles) {
    const ts = parseInt(pf.split('/').pop()?.split('_')[0] || '0', 10)
    if (ts < cutoff) { try { unlinkSync(pf) } catch { } }
  }

  return { merged, skipped }
}
