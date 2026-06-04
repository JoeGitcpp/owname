import { randomBytes } from 'node:crypto'

export function normalizeEmail(email) {
  const s = (email ?? '').trim().toLowerCase()
  if (!s.includes('@')) throw new Error('invalid email')
  return s
}

export function normalizePhone(phone) {
  const s = (phone ?? '').trim()
  const cleaned = s.replace(/[^\d+]/g, '').replace(/(?!^)\+/g, '')
  if (cleaned.replace('+', '').length < 7) throw new Error('invalid phone')
  return cleaned.startsWith('+') ? cleaned : '+' + cleaned
}

export function identityKey(type, value) {
  if (type !== 'email' && type !== 'phone') throw new Error(`unknown identity type: ${type}`)
  return `${type}:${value}`
}

export class OtpStore {
  #otps     = new Map()  // key → { otp, expiresAt, attempts, requestCount, windowStart }
  #sessions = new Map()  // token → { type, value, createdAt }
  #opts

  constructor({
    otpTtlMs     = 600_000,
    maxAttempts  = 3,
    maxRequests  = 3,
    windowMs     = 600_000,
    sessionTtlMs = 7 * 24 * 3_600_000,
    clock        = Date.now,
  } = {}) {
    this.#opts = { otpTtlMs, maxAttempts, maxRequests, windowMs, sessionTtlMs, clock }
  }

  async requestOtp(type, value, sender) {
    const now = this.#opts.clock()
    const key = identityKey(type, value)
    const entry = this.#otps.get(key) ?? {
      otp: null, expiresAt: 0, attempts: 0, requestCount: 0, windowStart: now,
    }

    if (now - entry.windowStart >= this.#opts.windowMs) {
      entry.requestCount = 0
      entry.windowStart  = now
    }

    if (entry.requestCount >= this.#opts.maxRequests) {
      throw new Error('rate limit exceeded')
    }

    // 6-digit OTP, uniform range 100000–999999
    const rand = randomBytes(4).readUInt32LE() % 900_000
    const otp  = String(100_000 + rand).padStart(6, '0')

    entry.otp        = otp
    entry.expiresAt  = now + this.#opts.otpTtlMs
    entry.attempts   = 0
    entry.requestCount++
    this.#otps.set(key, entry)

    await sender(value, otp)
  }

  verifyOtp(type, value, otp) {
    const now   = this.#opts.clock()
    const key   = identityKey(type, value)
    const entry = this.#otps.get(key)

    if (!entry?.otp)                          return null
    if (now > entry.expiresAt)                return null
    if (entry.attempts >= this.#opts.maxAttempts) return null

    if (entry.otp !== otp) {
      entry.attempts++
      return null
    }

    entry.otp = null  // single-use
    const token = randomBytes(32).toString('hex')
    this.#sessions.set(token, { type, value, createdAt: now })
    return token
  }

  checkSession(token) {
    const now = this.#opts.clock()
    const s   = this.#sessions.get(token)
    if (!s) return null
    if (now - s.createdAt > this.#opts.sessionTtlMs) {
      this.#sessions.delete(token)
      return null
    }
    return { type: s.type, value: s.value }
  }
}
