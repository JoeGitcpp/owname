/**
 * instance_registry.test.mjs
 * Run: node --test gateway/instance_registry.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { rmSync, existsSync, writeFileSync } from 'node:fs'
import { randomBytes } from 'node:crypto'
import { InstanceRegistry } from './instance_registry.mjs'

describe('InstanceRegistry', () => {
  it('create returns an object with id and pending_verification state', () => {
    const reg = new InstanceRegistry()
    const inst = reg.create('email:a@b.com')
    assert.ok(typeof inst.id === 'string' && inst.id.length > 0)
    assert.equal(inst.state, 'pending_verification')
    assert.equal(inst.identityKey, 'email:a@b.com')
    assert.ok(typeof inst.createdAt === 'number')
  })

  it('create throws if the same identity already has an active instance', () => {
    const reg = new InstanceRegistry()
    reg.create('email:a@b.com')
    assert.throws(() => reg.create('email:a@b.com'), /already/i)
  })

  it('create allows a new instance after the previous one is destroyed', () => {
    const reg = new InstanceRegistry()
    const inst = reg.create('email:a@b.com')
    reg.destroy(inst.id)
    assert.doesNotThrow(() => reg.create('email:a@b.com'))
  })

  it('get returns a copy of instance data by id', () => {
    const reg = new InstanceRegistry()
    const inst = reg.create('email:a@b.com')
    assert.equal(reg.get(inst.id).id, inst.id)
  })

  it('get returns null for an unknown id', () => {
    assert.equal(new InstanceRegistry().get('no-such-id'), null)
  })

  it('getByIdentity returns the instance for a known identity key', () => {
    const reg = new InstanceRegistry()
    const inst = reg.create('email:a@b.com')
    assert.equal(reg.getByIdentity('email:a@b.com').id, inst.id)
  })

  it('getByIdentity returns null for an unknown identity key', () => {
    assert.equal(new InstanceRegistry().getByIdentity('email:ghost@b.com'), null)
  })

  it('destroy removes the instance and frees the identity slot', () => {
    const reg = new InstanceRegistry()
    const inst = reg.create('email:a@b.com')
    reg.destroy(inst.id)
    assert.equal(reg.get(inst.id), null)
    assert.equal(reg.getByIdentity('email:a@b.com'), null)
  })

  it('destroy is a no-op for an unknown id', () => {
    assert.doesNotThrow(() => new InstanceRegistry().destroy('phantom'))
  })

  it('count tracks creates and destroys', () => {
    const reg = new InstanceRegistry()
    assert.equal(reg.count(), 0)
    const a = reg.create('email:a@b.com')
    reg.create('email:b@b.com')
    assert.equal(reg.count(), 2)
    reg.destroy(a.id)
    assert.equal(reg.count(), 1)
  })

  it('list returns all active instances', () => {
    const reg = new InstanceRegistry()
    reg.create('email:a@b.com')
    reg.create('phone:+15550001111')
    assert.equal(reg.list().length, 2)
  })

  it('different identity keys are fully independent', () => {
    const reg = new InstanceRegistry()
    const a = reg.create('email:a@b.com')
    const b = reg.create('phone:+15550001111')
    assert.notEqual(a.id, b.id)
    assert.equal(reg.count(), 2)
  })

  it('setState updates the instance state', () => {
    const reg = new InstanceRegistry()
    const inst = reg.create('email:a@b.com')
    reg.setState(inst.id, 'running')
    assert.equal(reg.get(inst.id).state, 'running')
  })
})

describe('InstanceRegistry persistence', () => {
  const path = join(tmpdir(), `reg-test-${randomBytes(6).toString('hex')}.json`)
  const cleanup = () => { try { rmSync(path, { force: true }) } catch {} }

  it('survives a restart: a new registry loads the persisted instances', () => {
    cleanup()
    const r1 = new InstanceRegistry(path)
    const inst = r1.create('email:user@aborn.live', 'my-ai')
    r1.setState(inst.id, 'running')
    assert.ok(existsSync(path), 'registry file written')

    // Simulate gateway restart: a brand-new registry over the same file.
    const r2 = new InstanceRegistry(path)
    const back = r2.getByIdentity('email:user@aborn.live')
    assert.ok(back, 'returning identity resolves to its persisted instance')
    assert.equal(back.id, inst.id, 'same instance id across restart')
    assert.equal(back.state, 'running', 'state persisted')
    assert.equal(back.name, 'my-ai', 'instance name persisted')
    cleanup()
  })

  it('destroy is persisted (not resurrected on reload)', () => {
    cleanup()
    const r1 = new InstanceRegistry(path)
    const inst = r1.create('phone:+15551234567')
    r1.destroy(inst.id)
    const r2 = new InstanceRegistry(path)
    assert.equal(r2.getByIdentity('phone:+15551234567'), null, 'destroyed instance stays gone')
    cleanup()
  })

  it('setName updates and persists', () => {
    cleanup()
    const r1 = new InstanceRegistry(path)
    const inst = r1.create('email:x@y.z')
    r1.setName(inst.id, 'renamed')
    assert.equal(new InstanceRegistry(path).get(inst.id).name, 'renamed')
    cleanup()
  })

  it('corrupt persisted file → starts empty, does not throw', () => {
    cleanup()
    writeFileSync(path, 'not json at all {{{')
    const r = new InstanceRegistry(path)
    assert.equal(r.count(), 0, 'corrupt registry file yields an empty registry')
    cleanup()
  })
})
