/**
 * security.mjs — gateway security primitives.
 *
 * Pure / side-effect-free so every function is testable in isolation.
 * server.mjs imports these and calls them at the right points.
 */

// ── Frame validation ──────────────────────────────────────────────────────────
// Compact binary frame constants (mirrored from net_adapter.rs).
// Only valid tag is 0x01 (senses). Exact size must match.
const TAG_SENSES   = 0x01
const BANDS        = 12
const GW           = 32
const GH           = 24
// 1 tag + (audio_level:f32 + audio_flux:f32 + 12×bands:f32 + motion:f32 + luma:f32) + grid:768×f32
// = 1 + (2+12)×4 + 32×24×4 = 1 + 56 + 3072 = 3129
export const SENSES_FRAME_BYTES = 1 + (2 + BANDS) * 4 + GW * GH * 4   // 3129

/**
 * Validate a binary frame from the browser before forwarding to body.
 * Returns true only if the frame is a well-formed senses packet.
 * Rejects anything else: wrong tag, wrong size, non-buffer, null.
 */
export function validateFrame(buf) {
  if (!buf || !Buffer.isBuffer(buf)) return false
  if (buf.length === 0) return false
  const tag = buf[0]
  if (tag === TAG_SENSES) return buf.length === SENSES_FRAME_BYTES
  return false  // any other tag or padding: reject
}

// ── Per-IP rate limiter ───────────────────────────────────────────────────────
/**
 * Sliding-window rate limiter: at most `maxPerMinute` WebSocket upgrade
 * attempts per IP address within `windowMs` milliseconds.
 *
 * Usage:
 *   const rl = new RateLimiter({ maxPerMinute: 5, windowMs: 60_000 })
 *   if (!rl.allow(ip)) { reject } else { proceed }
 */
export class RateLimiter {
  #maxPerMinute
  #windowMs
  #counters = new Map()  // ip → { count, windowStart }

  constructor({ maxPerMinute = 5, windowMs = 60_000 } = {}) {
    this.#maxPerMinute = maxPerMinute
    this.#windowMs     = windowMs
  }

  allow(ip) {
    if (!ip) return false
    const now  = Date.now()
    let entry  = this.#counters.get(ip)
    if (!entry || now - entry.windowStart >= this.#windowMs) {
      entry = { count: 0, windowStart: now }
      this.#counters.set(ip, entry)
    }
    if (entry.count >= this.#maxPerMinute) return false
    entry.count++
    return true
  }

  /** Test helper: force-reset the window for a given IP. */
  _resetFor(ip) {
    this.#counters.delete(ip)
  }

  /** Periodic cleanup to prevent unbounded memory growth. */
  prune() {
    const cutoff = Date.now() - this.#windowMs
    for (const [ip, entry] of this.#counters) {
      if (entry.windowStart < cutoff) this.#counters.delete(ip)
    }
  }
}

// ── Session guard ─────────────────────────────────────────────────────────────
/**
 * Tracks active sessions: enforces a global cap and a per-IP cap.
 * Each admitted session gets a unique string id.
 *
 * Usage:
 *   const sg = new SessionGuard({ globalMax: 8, perIpMax: 2 })
 *   const id = sg.admit(ip)   // returns session id string or false
 *   sg.release(id)             // call on disconnect
 */
export class SessionGuard {
  #globalMax
  #perIpMax
  #sessions = new Map()   // sessionId → ip
  #ipCounts  = new Map()  // ip → count
  #counter   = 0

  constructor({ globalMax = 8, perIpMax = 2 } = {}) {
    this.#globalMax = globalMax
    this.#perIpMax  = perIpMax
  }

  admit(ip) {
    if (!ip) return false
    if (this.#sessions.size >= this.#globalMax) return false
    const current = this.#ipCounts.get(ip) || 0
    if (current >= this.#perIpMax) return false
    const id = `${Date.now()}_${++this.#counter}`
    this.#sessions.set(id, ip)
    this.#ipCounts.set(ip, current + 1)
    return id
  }

  release(id) {
    const ip = this.#sessions.get(id)
    if (!ip) return
    this.#sessions.delete(id)
    const n = (this.#ipCounts.get(ip) || 1) - 1
    if (n <= 0) this.#ipCounts.delete(ip)
    else        this.#ipCounts.set(ip, n)
  }

  total()       { return this.#sessions.size }
  forIp(ip)     { return this.#ipCounts.get(ip) || 0 }
  sessions()    { return [...this.#sessions.keys()] }
}

// ── Idle-timeout tracker ──────────────────────────────────────────────────────
/**
 * Tracks the last-activity timestamp for a session.  The caller should call
 * touch(id) whenever a valid frame arrives, and check isIdle(id) periodically.
 */
export class IdleTracker {
  #timeouts = new Map()  // sessionId → lastActiveMs
  #idleMs

  constructor({ idleMs = 5 * 60_000 } = {}) {
    this.#idleMs = idleMs
  }

  add(id)    { this.#timeouts.set(id, Date.now()) }
  touch(id)  { if (this.#timeouts.has(id)) this.#timeouts.set(id, Date.now()) }
  remove(id) { this.#timeouts.delete(id) }

  isIdle(id) {
    const t = this.#timeouts.get(id)
    return t == null || Date.now() - t >= this.#idleMs
  }

  idleSessions() {
    return [...this.#timeouts.entries()]
      .filter(([, t]) => Date.now() - t >= this.#idleMs)
      .map(([id]) => id)
  }
}
