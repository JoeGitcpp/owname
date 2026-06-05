/**
 * harvest_pipeline.mjs — collect training outcomes + merge into Prime body.
 *
 * Relaxed quality gates vs training_pool.mjs defaults:
 *   MIN_SESSION_SECS: 300 → 60   (1 min minimum, was 5 min)
 *   MIN_RECOG: 20 → not checked  (bias delta is the main signal)
 *   Delta-only path: 60-300s sessions with real bias change qualify
 */
import { join }  from 'node:path'
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import { evaluate_session, collect_to_pool, merge_pool } from './training_pool.mjs'

const RELAXED_MIN_SECS = 60
const DELTA_THRESHOLD  = 0.005

export class HarvestPipeline {
  #poolDir
  #primeMgr
  #mergeIntervalMs
  #clock
  #totalMerges = 0
  #lastMergeAt = 0
  #mergeTimer = null

  constructor({ poolDir, primeMgr = null, mergeIntervalMs = 120_000, clock } = {}) {
    this.#poolDir        = poolDir
    this.#primeMgr       = primeMgr
    this.#mergeIntervalMs = mergeIntervalMs
    this.#clock = clock ?? (() => globalThis.Date ? globalThis.Date.now() : 0)
    if (mergeIntervalMs > 0) this.#startAutoMerge()
  }

  /**
   * Evaluate a finished session with relaxed gates.
   * Short sessions (60–300s) qualify if they have real bias delta.
   * Long sessions (≥300s) use the original strict evaluator.
   */
  evaluate(runtimeDir, startedAt, endedAt) {
    const secs = (endedAt - startedAt) / 1000

    if (secs < RELAXED_MIN_SECS) {
      return { pass: false, score: 0, reason: 'too_short' }
    }

    if (secs < 300) {
      // Short session: qualify only if there's real bias delta
      try {
        const lb = readFileSync(join(runtimeDir, 'learned_bias.bin'))
        const ab = lb.buffer.slice(lb.byteOffset, lb.byteOffset + lb.byteLength)
        const dv = new DataView(ab)
        let hasRealDelta = false
        for (let off = 28; off + 8 <= lb.length; off += 8) {
          const v = Math.abs(dv.getFloat64(off, true))
          if (v > DELTA_THRESHOLD && v < 1e6) { hasRealDelta = true; break }
        }
        if (hasRealDelta) return { pass: true, score: 0.3, reason: 'delta_only_relaxed_gate' }
        return { pass: false, score: 0, reason: 'no_delta' }
      } catch {
        return { pass: false, score: 0, reason: 'no_bias_file' }
      }
    }

    // Long session: use original evaluator (strict gates)
    return evaluate_session({ runtimeDir, startedAt, endedAt })
  }

  /** Collect a session's learned_bias into the pool if it passes the gate. */
  collect(runtimeDir, sessionId, startedAt, endedAt) {
    const result = this.evaluate(runtimeDir, startedAt, endedAt)
    if (!result.pass) return null
    return collect_to_pool(runtimeDir, this.#poolDir, sessionId)
  }

  /** Immediately merge the pool into the prime body's learned_bias. */
  async mergeNow() {
    const masterPath = this.#primeMgr ? this.#primeMgr.getMasterBiasPath() : null
    if (!masterPath) return { merged: 0, skipped: 0 }
    try {
      const result = merge_pool(this.#poolDir, masterPath)
      if (result.merged > 0) {
        this.#totalMerges += result.merged
        this.#lastMergeAt = this.#clock()
      }
      return result
    } catch {
      return { merged: 0, skipped: 0 }
    }
  }

  stats() {
    const poolSize = existsSync(this.#poolDir)
      ? readdirSync(this.#poolDir).filter(f => f.endsWith('.bin')).length
      : 0
    return {
      poolSize,
      totalMerges:  this.#totalMerges,
      lastMergeAt:  this.#lastMergeAt || null,
    }
  }

  stopAutoMerge() {
    if (this.#mergeTimer) { clearInterval(this.#mergeTimer); this.#mergeTimer = null }
  }

  #startAutoMerge() {
    this.#mergeTimer = setInterval(() => { this.mergeNow().catch(() => {}) }, this.#mergeIntervalMs)
  }
}
