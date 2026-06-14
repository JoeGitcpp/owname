import { randomBytes } from 'node:crypto'

/**
 * security.mjs — gateway security primitives.
 *
 * Pure / side-effect-free so every function is testable in isolation.
 * server.mjs imports these and calls them at the right points.
 */

// ── Frame validation ──────────────────────────────────────────────────────────
// Compact binary frame constants (mirrored from net_adapter.rs).
// Only valid tag is 0x01 (senses). Exact size must match.
const TAG_SENSES         = 0x01
export const TAG_SENSES_COMPACT = 0x06
const AUDIO_SCALARS      = 2 + 64 + 2 + 8   // audio_level, audio_flux, 64 mel, motion, luma, 8 aux
const AUDIO_BYTES        = AUDIO_SCALARS * 4 // 304 — the f32 scalar block, identical in both frames
// Legacy full senses frame (gateway→body, local socket): 160×120×3 f32 grid.
// tag(1) + 76 f32 scalars (304) + 160×120×3 × f32 (230400) = 230705
const FULL_GW = 160, FULL_GH = 120
export const SENSES_FRAME_BYTES = 1 + AUDIO_BYTES + FULL_GW * FULL_GH * 3 * 4   // 230705
// Compact uplink frame (browser→gateway, Cloudflare-tunnel-safe). The 230 KB
// float32 frame was dropped by the tunnel → body blind → black screen. The
// browser now sends a 40×30 u8 grid (downsampled 4×) + the SAME 76 f32 scalars;
// the gateway expands it back to the legacy 0x01 frame for the (local) body
// socket, so the body and the nursery feeder stay byte-for-byte unchanged.
const COMPACT_CW = 40, COMPACT_CH = 30, COMPACT_DS = 4   // 160/40 = 120/30 = 4
export const COMPACT_SENSES_BYTES = 1 + AUDIO_BYTES + COMPACT_CW * COMPACT_CH * 3  // 3905

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
  if (tag === TAG_SENSES_COMPACT) return buf.length === COMPACT_SENSES_BYTES
  return false  // any other tag or padding: reject
}

/**
 * Expand a compact 0x06 senses frame (40×30 u8 grid) into the legacy 0x01 frame
 * (160×120 f32 grid) the body expects: nearest-neighbour 4× upsample + u8→f32
 * (/255). The 76 audio/aux f32 scalars are copied verbatim. Returns a Buffer of
 * SENSES_FRAME_BYTES. `buf` must already be XOR-decrypted and validateFrame-passed
 * as a compact frame. Pure — the gateway calls it before writing to the body
 * socket; the body never sees the compact form.
 */
export function expandCompactSenses(buf) {
  const out = Buffer.allocUnsafe(SENSES_FRAME_BYTES)
  out[0] = TAG_SENSES                          // body expects 0x01
  buf.copy(out, 1, 1, 1 + AUDIO_BYTES)         // 304 scalar bytes verbatim
  const srcGrid = 1 + AUDIO_BYTES              // 305: u8 grid start in compact
  const plane = COMPACT_CW * COMPACT_CH        // 1200 u8 per channel
  let dst = 1 + AUDIO_BYTES                     // 305: f32 grid start in full
  for (let c = 0; c < 3; c++) {
    const base = srcGrid + c * plane
    for (let y = 0; y < FULL_GH; y++) {
      const row = base + ((y / COMPACT_DS) | 0) * COMPACT_CW
      for (let x = 0; x < FULL_GW; x++) {
        out.writeFloatLE(buf[row + ((x / COMPACT_DS) | 0)] / 255, dst)
        dst += 4
      }
    }
  }
  return out
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

// ── XOR session cipher ───────────────────────────────────────────────────────
/**
 * Lightweight per-session XOR obfuscation.
 * NOT cryptographic security — TLS (WSS) handles that.
 * Purpose: make the WebSocket payload completely unreadable in browser
 * DevTools / packet analysers so protocol structure cannot be reverse-
 * engineered by inspecting live traffic.
 *
 * Protocol:
 *   1. On WS connect the server sends a 5-byte hello:
 *        [0x00][key0][key1][key2][key3]
 *   2. All subsequent frames from the browser are XOR'd with the key
 *        (cycling through key[i % 4]).
 *   3. All frames sent TO the browser are XOR'd with the same key.
 */
export class XorCipher {
  /** Generate a fresh 4-byte random session key (synchronous). */
  static newKey() {
    return randomBytes(4)
  }

  /** Apply XOR cipher (in-place on a copy). Pure: returns a new Buffer. */
  static apply(data, key) {
    if (!Buffer.isBuffer(data) || !Buffer.isBuffer(key) || key.length < 4) {
      throw new Error('XorCipher.apply: invalid args')
    }
    const out = Buffer.allocUnsafe(data.length)
    for (let i = 0; i < data.length; i++) out[i] = data[i] ^ key[i % 4]
    return out
  }
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
