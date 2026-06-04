/**
 * memory_store.test.mjs
 * Run: node --test gateway/memory_store.test.mjs
 */
import { describe, it, after } from 'node:test'
import assert from 'node:assert/strict'
import { tmpdir } from 'node:os'
import { join }   from 'node:path'
import { rmSync } from 'node:fs'
import { MemoryStore } from './memory_store.mjs'

const BASE = join(tmpdir(), `memory-store-test-${process.pid}`)
after(() => { try { rmSync(BASE, { recursive: true, force: true }) } catch {} })

describe('MemoryStore', () => {
  it('write + read round-trips a scalar value', () => {
    const s = new MemoryStore(BASE)
    s.write('email:scalar@b.com', 'name', 'Alice')
    assert.equal(s.read('email:scalar@b.com', 'name'), 'Alice')
  })

  it('write + read round-trips an object', () => {
    const s = new MemoryStore(BASE)
    s.write('email:obj@b.com', 'prefs', { theme: 'dark', lang: 'zh' })
    assert.deepEqual(s.read('email:obj@b.com', 'prefs'), { theme: 'dark', lang: 'zh' })
  })

  it('read returns null for an unknown key', () => {
    const s = new MemoryStore(BASE)
    assert.equal(s.read('email:obj@b.com', 'no-such-key-xyz'), null)
  })

  it('different identity namespaces are isolated', () => {
    const s = new MemoryStore(BASE)
    s.write('email:alice@b.com', 'secret', 'for-alice')
    s.write('email:bob@b.com',   'secret', 'for-bob')
    assert.equal(s.read('email:alice@b.com', 'secret'), 'for-alice')
    assert.equal(s.read('email:bob@b.com',   'secret'), 'for-bob')
  })

  it('listKeys returns all written keys for an identity', () => {
    const s = new MemoryStore(BASE)
    s.write('phone:+15551111111', 'k1', 1)
    s.write('phone:+15551111111', 'k2', 2)
    const keys = s.listKeys('phone:+15551111111')
    assert.ok(keys.includes('k1'), 'missing k1')
    assert.ok(keys.includes('k2'), 'missing k2')
  })

  it('listKeys returns an empty array for an unknown identity', () => {
    const s = new MemoryStore(BASE)
    assert.deepEqual(s.listKeys('email:nobody@b.com'), [])
  })

  it('listKeys does not bleed into other namespaces', () => {
    const s = new MemoryStore(BASE)
    s.write('email:isolated@b.com', 'my-key', 42)
    const keys = s.listKeys('email:different@b.com')
    assert.ok(!keys.includes('my-key'))
  })

  it('deleteNamespace removes all keys for that identity', () => {
    const s = new MemoryStore(BASE)
    s.write('email:todelete@b.com', 'x', 1)
    s.write('email:todelete@b.com', 'y', 2)
    s.deleteNamespace('email:todelete@b.com')
    assert.equal(s.read('email:todelete@b.com', 'x'), null)
    assert.deepEqual(s.listKeys('email:todelete@b.com'), [])
  })

  it('deleteNamespace does not affect other identities', () => {
    const s = new MemoryStore(BASE)
    s.write('email:keep@b.com',   'data', 'safe')
    s.write('email:remove@b.com', 'data', 'gone')
    s.deleteNamespace('email:remove@b.com')
    assert.equal(s.read('email:keep@b.com', 'data'), 'safe')
  })

  it('harvest returns a complete snapshot of all keys', () => {
    const s = new MemoryStore(BASE)
    s.write('email:harvest@b.com', 'a', 1)
    s.write('email:harvest@b.com', 'b', 2)
    const snap = s.harvest('email:harvest@b.com')
    assert.equal(snap['a'], 1)
    assert.equal(snap['b'], 2)
  })

  it('harvest is non-destructive (data still readable afterward)', () => {
    const s = new MemoryStore(BASE)
    s.write('email:nd@b.com', 'val', 99)
    s.harvest('email:nd@b.com')
    assert.equal(s.read('email:nd@b.com', 'val'), 99)
  })

  it('data persists across separate MemoryStore instances with the same basePath', () => {
    const sub = join(BASE, 'persist-sub')
    const s1 = new MemoryStore(sub)
    s1.write('email:persist@b.com', 'survive', true)
    const s2 = new MemoryStore(sub)
    assert.equal(s2.read('email:persist@b.com', 'survive'), true)
  })
})
