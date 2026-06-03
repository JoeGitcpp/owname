/**
 * security.test.mjs — TDD for gateway security primitives.
 * Run: node --test gateway/security.test.mjs
 */
import { describe, it, before, beforeEach } from 'node:test'
import assert from 'node:assert/strict'
import { validateFrame, RateLimiter, SessionGuard } from './security.mjs'

// ── validateFrame ─────────────────────────────────────────────────────────────
const TAG_SENSES = 0x01
const FRAME_BYTES = 3129   // 1 + (2+12)*4 + 32*24*4 = 1+56+3072
const TAG_VOCAL  = 0x02

describe('validateFrame', () => {
  it('accepts a correctly-sized senses frame', () => {
    const buf = Buffer.alloc(FRAME_BYTES)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), true)
  })

  it('rejects wrong tag', () => {
    const buf = Buffer.alloc(FRAME_BYTES)
    buf[0] = 0xFF  // unknown tag
    assert.equal(validateFrame(buf), false)
  })

  it('rejects senses frame that is too short', () => {
    const buf = Buffer.alloc(FRAME_BYTES - 1)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), false)
  })

  it('rejects senses frame that is too large (possible padding attack)', () => {
    const buf = Buffer.alloc(FRAME_BYTES + 1000)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), false)
  })

  it('rejects empty buffer', () => {
    assert.equal(validateFrame(Buffer.alloc(0)), false)
  })

  it('rejects null / non-buffer', () => {
    assert.equal(validateFrame(null), false)
    assert.equal(validateFrame('attack'), false)
    assert.equal(validateFrame(undefined), false)
  })
})

// ── RateLimiter ───────────────────────────────────────────────────────────────
describe('RateLimiter', () => {
  it('allows connections within the limit', () => {
    const rl = new RateLimiter({ maxPerMinute: 3, windowMs: 60_000 })
    assert.equal(rl.allow('1.2.3.4'), true)
    assert.equal(rl.allow('1.2.3.4'), true)
    assert.equal(rl.allow('1.2.3.4'), true)
  })

  it('blocks after limit is exceeded', () => {
    const rl = new RateLimiter({ maxPerMinute: 2, windowMs: 60_000 })
    rl.allow('1.2.3.4')
    rl.allow('1.2.3.4')
    assert.equal(rl.allow('1.2.3.4'), false)
  })

  it('different IPs have independent counters', () => {
    const rl = new RateLimiter({ maxPerMinute: 1, windowMs: 60_000 })
    rl.allow('1.1.1.1')
    assert.equal(rl.allow('1.1.1.1'), false)  // 1.1.1.1 blocked
    assert.equal(rl.allow('2.2.2.2'), true)   // 2.2.2.2 allowed
  })

  it('window resets after windowMs (simulated via fake clock)', () => {
    const rl = new RateLimiter({ maxPerMinute: 1, windowMs: 100 })
    rl.allow('1.2.3.4')
    assert.equal(rl.allow('1.2.3.4'), false)
    // Simulate time passing by manually expiring the window.
    rl._resetFor('1.2.3.4')
    assert.equal(rl.allow('1.2.3.4'), true)
  })
})

// ── SessionGuard ──────────────────────────────────────────────────────────────
describe('SessionGuard', () => {
  it('allows sessions within global and per-ip limits', () => {
    // admit() returns a session-id string on success, false on rejection
    const sg = new SessionGuard({ globalMax: 4, perIpMax: 2 })
    assert.ok(sg.admit('1.1.1.1'))
    assert.ok(sg.admit('1.1.1.1'))
  })

  it('blocks when per-IP limit exceeded', () => {
    const sg = new SessionGuard({ globalMax: 10, perIpMax: 1 })
    assert.ok(sg.admit('1.1.1.1'))
    assert.equal(sg.admit('1.1.1.1'), false)
  })

  it('blocks when global limit exceeded', () => {
    const sg = new SessionGuard({ globalMax: 2, perIpMax: 10 })
    assert.ok(sg.admit('1.1.1.1'))
    assert.ok(sg.admit('2.2.2.2'))
    assert.equal(sg.admit('3.3.3.3'), false)  // global full
  })

  it('release frees a slot', () => {
    const sg = new SessionGuard({ globalMax: 1, perIpMax: 1 })
    const id = sg.admit('1.1.1.1')
    assert.ok(id)
    assert.equal(sg.admit('2.2.2.2'), false)  // full
    sg.release(id)
    assert.ok(sg.admit('2.2.2.2'))  // freed
  })

  it('total() and forIp() reflect current counts', () => {
    const sg = new SessionGuard({ globalMax: 10, perIpMax: 10 })
    sg.admit('1.1.1.1')
    sg.admit('1.1.1.1')
    sg.admit('2.2.2.2')
    assert.equal(sg.total(), 3)
    assert.equal(sg.forIp('1.1.1.1'), 2)
    assert.equal(sg.forIp('2.2.2.2'), 1)
    assert.equal(sg.forIp('3.3.3.3'), 0)
  })
})
