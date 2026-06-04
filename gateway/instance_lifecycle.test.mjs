/**
 * instance_lifecycle.test.mjs
 * Run: node --test gateway/instance_lifecycle.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { transition, STATES } from './instance_lifecycle.mjs'

describe('transition — valid paths', () => {
  it('pending_verification + verify → running', () => {
    assert.equal(transition('pending_verification', 'verify'), 'running')
  })
  it('pending_verification + destroy → destroyed', () => {
    assert.equal(transition('pending_verification', 'destroy'), 'destroyed')
  })
  it('running + suspend → suspended', () => {
    assert.equal(transition('running', 'suspend'), 'suspended')
  })
  it('running + destroy → destroyed', () => {
    assert.equal(transition('running', 'destroy'), 'destroyed')
  })
  it('suspended + resume → running', () => {
    assert.equal(transition('suspended', 'resume'), 'running')
  })
  it('suspended + destroy → destroyed', () => {
    assert.equal(transition('suspended', 'destroy'), 'destroyed')
  })
})

describe('transition — invalid paths throw', () => {
  it('pending_verification + suspend → throws', () => {
    assert.throws(() => transition('pending_verification', 'suspend'), /invalid/i)
  })
  it('pending_verification + resume → throws', () => {
    assert.throws(() => transition('pending_verification', 'resume'), /invalid/i)
  })
  it('running + verify → throws (already past verification)', () => {
    assert.throws(() => transition('running', 'verify'), /invalid/i)
  })
  it('running + resume → throws', () => {
    assert.throws(() => transition('running', 'resume'), /invalid/i)
  })
  it('suspended + suspend → throws', () => {
    assert.throws(() => transition('suspended', 'suspend'), /invalid/i)
  })
  it('suspended + verify → throws', () => {
    assert.throws(() => transition('suspended', 'verify'), /invalid/i)
  })
  it('destroyed + resume → throws', () => {
    assert.throws(() => transition('destroyed', 'resume'), /invalid/i)
  })
  it('destroyed + destroy → throws', () => {
    assert.throws(() => transition('destroyed', 'destroy'), /invalid/i)
  })
  it('unknown state → throws', () => {
    assert.throws(() => transition('zombie', 'verify'), /unknown/i)
  })
})

describe('STATES', () => {
  it('exports all four lifecycle state names', () => {
    for (const s of ['pending_verification', 'running', 'suspended', 'destroyed']) {
      assert.ok(STATES.includes(s), `missing state: ${s}`)
    }
  })
})
