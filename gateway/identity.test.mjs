/**
 * identity.test.mjs
 * Run: node --test gateway/identity.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { normalizeEmail, normalizePhone, identityKey, OtpStore } from './identity.mjs'

describe('normalizeEmail', () => {
  it('lowercases and trims whitespace', () => {
    assert.equal(normalizeEmail('  FOO@BAR.COM  '), 'foo@bar.com')
  })
  it('rejects empty string', () => {
    assert.throws(() => normalizeEmail(''), /invalid/i)
  })
  it('rejects string without @', () => {
    assert.throws(() => normalizeEmail('notanemail'), /invalid/i)
  })
})

describe('normalizePhone', () => {
  it('strips formatting and returns E.164-style number', () => {
    assert.equal(normalizePhone('+1 (555) 123-4567'), '+15551234567')
  })
  it('preserves leading +', () => {
    assert.equal(normalizePhone('+8613800138000'), '+8613800138000')
  })
  it('rejects too-short input', () => {
    assert.throws(() => normalizePhone('123'), /invalid/i)
  })
})

describe('identityKey', () => {
  it('formats email type as "email:<addr>"', () => {
    assert.equal(identityKey('email', 'foo@bar.com'), 'email:foo@bar.com')
  })
  it('formats phone type as "phone:<num>"', () => {
    assert.equal(identityKey('phone', '+15551234567'), 'phone:+15551234567')
  })
  it('throws on unknown type', () => {
    assert.throws(() => identityKey('sms', 'foo'), /unknown/i)
  })
})

describe('OtpStore', () => {
  function makeStore(overrides = {}) {
    return new OtpStore({
      otpTtlMs:    600_000,
      maxAttempts: 3,
      maxRequests: 3,
      windowMs:    600_000,
      sessionTtlMs: 3_600_000,
      ...overrides,
    })
  }

  it('requestOtp calls sender with a 6-digit numeric code', async () => {
    const store = makeStore()
    let sent = null
    await store.requestOtp('email', 'a@b.com', (_addr, otp) => { sent = otp })
    assert.match(sent, /^\d{6}$/)
  })

  it('verifyOtp returns a session token on correct OTP', async () => {
    const store = makeStore()
    let otp
    await store.requestOtp('email', 'a@b.com', (_, o) => { otp = o })
    const token = store.verifyOtp('email', 'a@b.com', otp)
    assert.ok(typeof token === 'string' && token.length >= 16)
  })

  it('verifyOtp returns null on wrong OTP', async () => {
    const store = makeStore()
    await store.requestOtp('email', 'a@b.com', () => {})
    assert.equal(store.verifyOtp('email', 'a@b.com', '000000'), null)
  })

  it('checkSession returns identity for a valid token', async () => {
    const store = makeStore()
    let otp
    await store.requestOtp('email', 'a@b.com', (_, o) => { otp = o })
    const token = store.verifyOtp('email', 'a@b.com', otp)
    assert.deepEqual(store.checkSession(token), { type: 'email', value: 'a@b.com' })
  })

  it('checkSession returns null for an unknown token', () => {
    assert.equal(makeStore().checkSession('totally-fake'), null)
  })

  it('verifyOtp locks the entry after maxAttempts wrong guesses', async () => {
    const store = makeStore({ maxAttempts: 3 })
    await store.requestOtp('email', 'lock@b.com', () => {})
    store.verifyOtp('email', 'lock@b.com', '111111')
    store.verifyOtp('email', 'lock@b.com', '222222')
    store.verifyOtp('email', 'lock@b.com', '333333')
    // 4th attempt — locked regardless of value
    assert.equal(store.verifyOtp('email', 'lock@b.com', '999999'), null)
  })

  it('requestOtp throws when rate limit exceeded', async () => {
    const store = makeStore({ maxRequests: 2 })
    await store.requestOtp('email', 'rl@b.com', () => {})
    await store.requestOtp('email', 'rl@b.com', () => {})
    await assert.rejects(
      () => store.requestOtp('email', 'rl@b.com', () => {}),
      /rate/i
    )
  })

  it('verifyOtp returns null after OTP TTL expires (fake clock)', async () => {
    let now = 0
    const store = new OtpStore({ otpTtlMs: 1000, maxAttempts: 3, maxRequests: 10, windowMs: 60_000, clock: () => now })
    let otp
    await store.requestOtp('email', 'ttl@b.com', (_, o) => { otp = o })
    now = 1001
    assert.equal(store.verifyOtp('email', 'ttl@b.com', otp), null)
  })

  it('checkSession returns null after session TTL expires (fake clock)', async () => {
    let now = 0
    const store = new OtpStore({ otpTtlMs: 600_000, maxAttempts: 3, maxRequests: 10, windowMs: 60_000, sessionTtlMs: 1000, clock: () => now })
    let otp
    await store.requestOtp('email', 'sttl@b.com', (_, o) => { otp = o })
    const token = store.verifyOtp('email', 'sttl@b.com', otp)
    now = 1001
    assert.equal(store.checkSession(token), null)
  })

  it('phone identity works the same as email', async () => {
    const store = makeStore()
    let otp
    await store.requestOtp('phone', '+15550001111', (_, o) => { otp = o })
    const token = store.verifyOtp('phone', '+15550001111', otp)
    assert.deepEqual(store.checkSession(token), { type: 'phone', value: '+15550001111' })
  })
})
