/**
 * resource_governor.test.mjs
 * Run: node --test gateway/resource_governor.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { ResourceGovernor } from './resource_governor.mjs'

describe('ResourceGovernor', () => {
  it('canStart is true when below limit', () => {
    assert.equal(new ResourceGovernor(3).canStart(), true)
  })

  it('canStart is false when at limit', () => {
    const gov = new ResourceGovernor(2)
    gov.onStart('a')
    gov.onStart('b')
    assert.equal(gov.canStart(), false)
  })

  it('onStart throws when at limit', () => {
    const gov = new ResourceGovernor(1)
    gov.onStart('a')
    assert.throws(() => gov.onStart('b'), /limit/i)
  })

  it('onDestroy of a running instance frees a slot', () => {
    const gov = new ResourceGovernor(1)
    gov.onStart('a')
    assert.equal(gov.canStart(), false)
    gov.onDestroy('a')
    assert.equal(gov.canStart(), true)
  })

  it('onDestroy returns the oldest suspended instance when capacity opens', () => {
    const gov = new ResourceGovernor(1)
    gov.onStart('a')
    gov.onSuspend('b')
    const next = gov.onDestroy('a')
    assert.equal(next, 'b')
  })

  it('onDestroy returns null when the suspend queue is empty', () => {
    const gov = new ResourceGovernor(2)
    gov.onStart('a')
    assert.equal(gov.onDestroy('a'), null)
  })

  it('onSuspend moves an instance out of running and into the queue', () => {
    const gov = new ResourceGovernor(2)
    gov.onStart('a')
    gov.onSuspend('a')
    assert.equal(gov.running(), 0)
    assert.equal(gov.suspended(), 1)
  })

  it('suspendQueue preserves FIFO order', () => {
    const gov = new ResourceGovernor(3)
    gov.onStart('a')
    gov.onStart('b')
    gov.onSuspend('a')
    gov.onSuspend('b')
    assert.deepEqual(gov.suspendQueue(), ['a', 'b'])
  })

  it('setLimit upward unlocks additional slots', () => {
    const gov = new ResourceGovernor(1)
    gov.onStart('a')
    assert.equal(gov.canStart(), false)
    gov.setLimit(3)
    assert.equal(gov.canStart(), true)
    assert.equal(gov.limit, 3)
  })

  it('setLimit downward returns the excess running instances that must be suspended', () => {
    const gov = new ResourceGovernor(3)
    gov.onStart('a')
    gov.onStart('b')
    gov.onStart('c')
    const toSuspend = gov.setLimit(1)
    assert.equal(toSuspend.length, 2)
    assert.equal(gov.running(), 1)
    assert.equal(gov.suspended(), 2)
  })

  it('running() / suspended() / total() reflect current state', () => {
    const gov = new ResourceGovernor(3)
    gov.onStart('a')
    gov.onStart('b')
    gov.onSuspend('b')
    assert.equal(gov.running(), 1)
    assert.equal(gov.suspended(), 1)
    assert.equal(gov.total(), 2)
  })
})
