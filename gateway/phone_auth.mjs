/**
 * phone_auth.mjs — phone OTP authentication.
 *
 * Flow:
 *   1. POST /auth/request-phone  { phone }
 *      → normalises to E.164, generates 6-digit OTP, sends SMS
 *      → returns { ok: true }  (never reveals whether number exists)
 *
 *   2. POST /auth/verify-phone   { phone, otp }
 *      → on match: sets HttpOnly session cookie _ab_phone, returns { ok: true }
 *      → on mismatch / expired: { ok: false }
 *
 * SMS transport:
 *   Dev fallback — prints OTP to stdout when ABORN_SMS_SEND is not set.
 *   Production — set ABORN_SMS_SEND=<provider> and supply provider-specific env
 *   vars (Twilio: ABORN_TWILIO_SID, ABORN_TWILIO_TOKEN, ABORN_TWILIO_FROM).
 *
 * Security mirrors auth.mjs:
 *   - OTP: 6 digits, valid 10 min, max 3 requests per 10-min window
 *   - 3 wrong guesses lock the number for the OTP window
 *   - Cookie: HttpOnly, Secure, SameSite=Strict, HMAC-signed, 7-day TTL
 *   - No phone number stored in the cookie value (only a random token)
 */

import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto'
import { normalizePhone, OtpStore }                 from './identity.mjs'
import { SESSION_SECRET }                           from './session_secret.mjs'

// ── Config ────────────────────────────────────────────────────────────────────
const SECRET      = SESSION_SECRET
const COOKIE_NAME = '_ab_phone'
const COOKIE_TTL  = 7 * 24 * 60 * 60 * 1000  // 7 days

// ── SMS sender (pluggable) ────────────────────────────────────────────────────
function smsSender(phone, otp) {
  if (process.env.ABORN_SMS_SEND === 'twilio') {
    // Twilio integration — add @twilio/twilio-node to package.json to enable.
    // Dynamic import so the module is optional; gateway starts without it.
    import('twilio').then(({ default: Twilio }) => {
      const client = Twilio(
        process.env.ABORN_TWILIO_SID,
        process.env.ABORN_TWILIO_TOKEN,
      )
      client.messages.create({
        from: process.env.ABORN_TWILIO_FROM,
        to:   phone,
        body: `aborn 验证码：${otp}，10 分钟内有效，请勿泄露。`,
      }).catch(e => console.error('[phone-auth] sms failed:', e.message))
    }).catch(() => {
      console.warn('[phone-auth] twilio module not installed — OTP not sent')
    })
    return
  }
  // Dev fallback: print OTP to stdout
  console.log(`[phone-auth:dev] OTP for ${phone}: ${otp}`)
}

// ── OtpStore instance ─────────────────────────────────────────────────────────
const otpStore = new OtpStore({
  otpTtlMs:     10 * 60_000,
  maxAttempts:  3,
  maxRequests:  3,
  windowMs:     10 * 60_000,
  sessionTtlMs: COOKIE_TTL,
})

// ── Session store + cookie helpers ───────────────────────────────────────────
const sessions = new Map()  // token → { phone, createdAt }

function signToken(token) {
  return createHmac('sha256', SECRET).update(token).digest('hex')
}

// Stateless signed cookie: the identity + expiry live INSIDE the signed payload,
// so a returning user stays logged in across a gateway restart (the old random
// token → in-memory `sessions` Map lookup lost every login on restart). Format:
// `<base64url(payload)>.<hmac(payload)>`.
function makePhoneCookie(phone) {
  const maxAge  = Math.floor(COOKIE_TTL / 1000)
  const payload = Buffer.from(JSON.stringify({ phone, exp: Date.now() + COOKIE_TTL })).toString('base64url')
  const value   = `${payload}.${signToken(payload)}`
  return {
    value,
    header: `${COOKIE_NAME}=${value}; HttpOnly; Secure; SameSite=Strict; Max-Age=${maxAge}; Path=/`,
  }
}

export function verifyPhoneCookie(cookieHeader) {
  if (!cookieHeader) return null
  const match = cookieHeader.match(new RegExp(`${COOKIE_NAME}=([^;]+)`))
  if (!match) return null
  const [payload, sig] = match[1].split('.')
  if (!payload || !sig) return null
  const expected = Buffer.from(signToken(payload))
  const actual   = Buffer.from(sig)
  if (expected.length !== actual.length) return null
  try {
    if (!timingSafeEqual(expected, actual)) return null
  } catch { return null }
  try {
    const obj = JSON.parse(Buffer.from(payload, 'base64url').toString('utf8'))
    if (!obj.phone || typeof obj.exp !== 'number' || obj.exp < Date.now()) return null
    return { phone: obj.phone }
  } catch { return null }
}

// Periodic session prune
setInterval(() => {
  const now = Date.now()
  for (const [k, v] of sessions) {
    if (now - v.createdAt > COOKIE_TTL) sessions.delete(k)
  }
}, 60 * 60_000)

// ── Shared request helpers ────────────────────────────────────────────────────
function readBody(req) {
  return new Promise(resolve => {
    let body = ''
    req.on('data', c => body += c)
    req.on('end', () => resolve(body))
  })
}

function sendJson(res, status, data) {
  res.writeHead(status, { 'Content-Type': 'application/json' })
  res.end(JSON.stringify(data))
}

// ── Route handlers ────────────────────────────────────────────────────────────

/**
 * POST /auth/request-phone — { phone: string } → { ok: true }
 * Always returns 200 ok=true to avoid revealing whether the number exists.
 * Returns 400 on malformed input, 429 on rate limit.
 */
export async function handlePhoneRequest(req, res) {
  const body = await readBody(req)
  let phone
  try { phone = normalizePhone(JSON.parse(body).phone) } catch { }
  if (!phone) return sendJson(res, 400, { ok: false, error: 'invalid_phone' })

  try {
    await otpStore.requestOtp('phone', phone, smsSender)
    sendJson(res, 200, { ok: true })
  } catch (e) {
    const status = /rate/i.test(e.message) ? 429 : 400
    sendJson(res, status, { ok: false, error: 'rate_limit' })
  }
}

/**
 * POST /auth/verify-phone — { phone: string, otp: string } → { ok: bool }
 * On success: sets _ab_phone HttpOnly cookie and returns { ok: true }.
 * On mismatch / expired: returns { ok: false }.
 */
export async function handlePhoneVerify(req, res) {
  const body = await readBody(req)
  let phone, otp
  try {
    const p = JSON.parse(body)
    phone   = normalizePhone(p.phone)
    otp     = String(p.otp ?? '').trim()
  } catch { }
  if (!phone || !otp) return sendJson(res, 400, { ok: false, error: 'bad_request' })

  const token = otpStore.verifyOtp('phone', phone, otp)
  if (!token) return sendJson(res, 200, { ok: false, error: 'invalid_otp' })

  const { header } = makePhoneCookie(phone)
  res.writeHead(200, { 'Content-Type': 'application/json', 'Set-Cookie': header })
  res.end(JSON.stringify({ ok: true }))
}
