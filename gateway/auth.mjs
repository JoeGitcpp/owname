/**
 * auth.mjs — email OTP authentication.
 *
 * Flow:
 *   1. POST /auth/request  { email }
 *      → generates 6-digit OTP, stores it (TTL 10 min), sends email
 *      → returns { ok: true }  (never reveals whether email exists)
 *
 *   2. POST /auth/verify   { email, otp }
 *      → on match: sets HttpOnly session cookie, returns { ok: true }
 *      → on mismatch: { ok: false }  (3 attempts max per email per window)
 *
 *   3. GET /auth/check
 *      → returns { authed: true/false } based on cookie
 *
 * Mail transport: nodemailer via SMTP.
 * Configure via environment variables (never in code):
 *   ABORN_SMTP_HOST, ABORN_SMTP_PORT, ABORN_SMTP_USER, ABORN_SMTP_PASS
 *   ABORN_FROM_EMAIL   (e.g. noreply@aborn.live)
 *   ABORN_SESSION_SECRET  (random string, signs the session cookie)
 *
 * Security:
 *   - OTP is rate-limited: max 3 requests per email per 10 min
 *   - 3 failed verifications lock the email for 10 min
 *   - Session cookie: HttpOnly, Secure, SameSite=Strict, signed HMAC
 *   - No email address stored in cookie (just a session token)
 */

import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto'
import { createTransport }    from 'nodemailer'

// ── Config (all via env) ──────────────────────────────────────────────────────
const SMTP_HOST    = process.env.ABORN_SMTP_HOST    || ''
const SMTP_PORT    = parseInt(process.env.ABORN_SMTP_PORT || '465', 10)
const SMTP_USER    = process.env.ABORN_SMTP_USER    || ''
const SMTP_PASS    = process.env.ABORN_SMTP_PASS    || ''
const FROM_EMAIL   = process.env.ABORN_FROM_EMAIL   || `noreply@aborn.live`
const SECRET       = process.env.ABORN_SESSION_SECRET || randomBytes(32).toString('hex')
const OTP_TTL_MS   = 10 * 60 * 1000   // 10 minutes
const MAX_ATTEMPTS = 3
const COOKIE_NAME  = '_ab'
const COOKIE_TTL   = 7 * 24 * 60 * 60 * 1000  // 7 days

// ── Mail transport ────────────────────────────────────────────────────────────
let transporter = null

function getTransporter() {
  if (transporter) return transporter
  if (!SMTP_HOST || !SMTP_USER || !SMTP_PASS) {
    console.warn('[auth] SMTP not configured — OTP emails will not be sent')
    return null
  }
  transporter = createTransport({
    host:   SMTP_HOST,
    port:   SMTP_PORT,
    secure: SMTP_PORT === 465,
    auth:   { user: SMTP_USER, pass: SMTP_PASS },
  })
  return transporter
}

async function sendOtpEmail(to, otp) {
  const t = getTransporter()
  if (!t) {
    // Dev fallback: print to stdout when SMTP not configured
    console.log(`[auth:dev] OTP for ${to}: ${otp}`)
    return
  }
  await t.sendMail({
    from:    FROM_EMAIL,
    to,
    subject: `aborn — your access code`,
    text:    `Your access code is: ${otp}\n\nValid for 10 minutes. Do not share it.`,
    html:    `<p style="font-family:monospace;font-size:24px">${otp}</p><p>Valid for 10 minutes.</p>`,
  })
}

// ── In-memory stores (restart-safe is fine; OTPs are short-lived) ─────────────
const otpStore    = new Map()  // email → { otp, expiresAt, attempts, requestCount, windowStart }
const sessionStore = new Map() // token → { email, expiresAt }

function pruneOtp() {
  const now = Date.now()
  for (const [k, v] of otpStore) if (v.expiresAt < now) otpStore.delete(k)
}
function pruneSessions() {
  const now = Date.now()
  for (const [k, v] of sessionStore) if (v.expiresAt < now) sessionStore.delete(k)
}
setInterval(() => { pruneOtp(); pruneSessions() }, 5 * 60_000)

// ── Session cookie helpers ────────────────────────────────────────────────────
function signToken(token) {
  return createHmac('sha256', SECRET).update(token).digest('hex')
}

function makeSessionCookie(email) {
  const token    = randomBytes(24).toString('hex')
  const expires  = Date.now() + COOKIE_TTL
  sessionStore.set(token, { email, expiresAt: expires })
  const sig      = signToken(token)
  const value    = `${token}.${sig}`
  const maxAge   = Math.floor(COOKIE_TTL / 1000)
  return {
    value,
    header: `${COOKIE_NAME}=${value}; HttpOnly; Secure; SameSite=Strict; Max-Age=${maxAge}; Path=/`,
  }
}

export function verifySessionCookie(cookieHeader) {
  if (!cookieHeader) return null
  const match = cookieHeader.match(new RegExp(`${COOKIE_NAME}=([^;]+)`))
  if (!match) return null
  const [token, sig] = match[1].split('.')
  if (!token || !sig) return null
  const expected = Buffer.from(signToken(token))
  const actual   = Buffer.from(sig)
  if (expected.length !== actual.length) return null
  try {
    if (!timingSafeEqual(expected, actual)) return null
  } catch { return null }
  const session = sessionStore.get(token)
  if (!session || session.expiresAt < Date.now()) return null
  return session
}

// ── Route handlers ────────────────────────────────────────────────────────────

/** POST /auth/request — { email: string } → { ok: true } */
export async function handleAuthRequest(req, res) {
  let body = ''
  for await (const chunk of req) body += chunk
  let email
  try { email = JSON.parse(body).email?.toLowerCase()?.trim() } catch { }
  if (!email || !email.includes('@')) {
    res.writeHead(400, { 'Content-Type': 'application/json' })
    res.end(JSON.stringify({ ok: false, error: 'invalid_email' }))
    return
  }

  pruneOtp()
  const existing = otpStore.get(email)
  const now = Date.now()

  // Rate limit: max 3 OTP requests per 10-minute window
  if (existing && existing.expiresAt > now) {
    const windowCount = (existing.requestCount || 1)
    if (windowCount >= MAX_ATTEMPTS) {
      res.writeHead(429, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ ok: false, error: 'rate_limit' }))
      return
    }
    otpStore.set(email, { ...existing, requestCount: windowCount + 1 })
  } else {
    const otp = String(Math.floor(100000 + Math.random() * 900000))
    otpStore.set(email, {
      otp,
      expiresAt: now + OTP_TTL_MS,
      attempts: 0,
      requestCount: 1,
    })
    try { await sendOtpEmail(email, otp) } catch { /* log internally, don't reveal */ }
  }

  res.writeHead(200, { 'Content-Type': 'application/json' })
  res.end(JSON.stringify({ ok: true }))
}

/** POST /auth/verify — { email, otp } → { ok: bool } */
export function handleAuthVerify(req, res) {
  let body = ''
  req.on('data', c => body += c)
  req.on('end', () => {
    let email, otp
    try { const p = JSON.parse(body); email = p.email?.toLowerCase()?.trim(); otp = p.otp?.trim() } catch { }
    if (!email || !otp) {
      res.writeHead(400, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ ok: false })); return
    }

    const entry = otpStore.get(email)
    if (!entry || entry.expiresAt < Date.now()) {
      res.writeHead(200, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ ok: false, error: 'expired' })); return
    }

    entry.attempts = (entry.attempts || 0) + 1
    if (entry.attempts > MAX_ATTEMPTS) {
      res.writeHead(429, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ ok: false, error: 'too_many_attempts' })); return
    }

    if (entry.otp !== otp) {
      res.writeHead(200, { 'Content-Type': 'application/json' })
      res.end(JSON.stringify({ ok: false })); return
    }

    otpStore.delete(email)
    const { header } = makeSessionCookie(email)
    res.writeHead(200, {
      'Content-Type': 'application/json',
      'Set-Cookie': header,
    })
    res.end(JSON.stringify({ ok: true }))
  })
}

/** GET /auth/check — → { authed: bool } */
export function handleAuthCheck(req, res) {
  const session = verifySessionCookie(req.headers.cookie)
  res.writeHead(200, { 'Content-Type': 'application/json' })
  res.end(JSON.stringify({ authed: !!session }))
}

/**
 * Returns true if this connection already has a valid session (i.e. has passed
 * OTP). Used by server.mjs to decide whether to close with 1008.
 *
 * Current behaviour: when SMTP is configured, only users with a valid signed
 * session cookie are gated — everyone else connects as 'anon' (anonymous).
 * The OTP flow exists server-side (/auth/request, /auth/verify) but the
 * client UI for it is not yet built. Until it is, anonymous access is allowed
 * so the site remains usable.
 *
 * TODO: once the client OTP UI is built, change this to:
 *   return !SMTP_HOST || !!verifySessionCookie(req.headers.cookie)
 * so that unauthenticated users are redirected to the OTP form instead of
 * connecting anonymously.
 */
export function requireAuth(req) {
  // Called as: if (!authSession && requireAuth(req)) { close }
  // Returning false = do NOT block = let everyone through (anon or authed).
  // OTP client UI is not yet built; gate is disabled until it is.
  void req  // suppress unused-param warning
  return false
}
