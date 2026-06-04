/**
 * ephemeral_session.test.mjs
 * Run: node --test gateway/ephemeral_session.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { EphemeralSessionManager } from './ephemeral_session.mjs'

describe('EphemeralSessionManager', () => {
  it('create returns a non-empty session token', () => {
    const mgr = new EphemeralSessionManager()
    const id = mgr.create('1.2.3.4')
    assert.ok(typeof id === 'string' && id.length > 0)
  })

  it('create throws when the per-IP limit is reached', () => {
    const mgr = new EphemeralSessionManager({ perIpMax: 2 })
    mgr.create('1.2.3.4')
    mgr.create('1.2.3.4')
    assert.throws(() => mgr.create('1.2.3.4'), /limit/i)
  })

  it('different IPs have independent limits', () => {
    const mgr = new EphemeralSessionManager({ perIpMax: 1 })
    mgr.create('1.1.1.1')
    assert.throws(() => mgr.create('1.1.1.1'), /limit/i)
    assert.doesNotThrow(() => mgr.create('2.2.2.2'))
  })

  it('destroy frees a per-IP slot', () => {
    const mgr = new EphemeralSessionManager({ perIpMax: 1 })
    const id = mgr.create('1.1.1.1')
    assert.throws(() => mgr.create('1.1.1.1'), /limit/i)
    mgr.destroy(id)
    assert.doesNotThrow(() => mgr.create('1.1.1.1'))
  })

  it('destroy is a no-op for an unknown session', () => {
    assert.doesNotThrow(() => new EphemeralSessionManager().destroy('ghost'))
  })

  it('size() tracks total active sessions', () => {
    const mgr = new EphemeralSessionManager()
    assert.equal(mgr.size(), 0)
    const a = mgr.create('1.1.1.1')
    const b = mgr.create('2.2.2.2')
    assert.equal(mgr.size(), 2)
    mgr.destroy(a)
    assert.equal(mgr.size(), 1)
  })

  it('countForIp reflects per-IP session counts', () => {
    const mgr = new EphemeralSessionManager()
    mgr.create('1.1.1.1')
    mgr.create('1.1.1.1')
    mgr.create('2.2.2.2')
    assert.equal(mgr.countForIp('1.1.1.1'), 2)
    assert.equal(mgr.countForIp('2.2.2.2'), 1)
    assert.equal(mgr.countForIp('9.9.9.9'), 0)
  })

  it('session expires after timeoutMs with fake clock', () => {
    let now = 0
    const mgr = new EphemeralSessionManager({ timeoutMs: 1000, clock: () => now })
    mgr.create('1.1.1.1')
    assert.equal(mgr.size(), 1)
    now = 1001
    assert.equal(mgr.size(), 0)   // size() triggers _sweep
  })

  it('heartbeat resets the idle timer', () => {
    let now = 0
    const mgr = new EphemeralSessionManager({ timeoutMs: 1000, clock: () => now })
    const id = mgr.create('1.1.1.1')
    now = 900
    mgr.heartbeat(id)      // reset at t=900
    now = 1500             // 600 ms after heartbeat — still alive
    assert.equal(mgr.size(), 1)
    now = 1901             // 1001 ms after heartbeat — expired
    assert.equal(mgr.size(), 0)
  })

  it('heartbeat is a no-op for an unknown session', () => {
    assert.doesNotThrow(() => new EphemeralSessionManager().heartbeat('ghost'))
  })

  it('expired sessions are swept on create, freeing per-IP slots', () => {
    let now = 0
    const mgr = new EphemeralSessionManager({ perIpMax: 2, timeoutMs: 500, clock: () => now })
    mgr.create('1.1.1.1')
    mgr.create('1.1.1.1')
    assert.throws(() => mgr.create('1.1.1.1'), /limit/i)
    now = 600   // both sessions expired
    assert.doesNotThrow(() => mgr.create('1.1.1.1'))  // sweep clears old slots
  })
})
