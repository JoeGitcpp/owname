/**
 * harvest_pipeline.test.mjs
 * Run: node --test gateway/harvest_pipeline.test.mjs
 */
import { describe, it, after } from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, rmSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { HarvestPipeline } from './harvest_pipeline.mjs'

const BASE = join(tmpdir(), `hp-test-${process.pid}`)
mkdirSync(BASE, { recursive: true })
after(() => rmSync(BASE, { recursive: true, force: true }))

function dir(name) {
  const d = join(BASE, name)
  mkdirSync(d, { recursive: true })
  return d
}

// Write a minimal learned_bias.bin with or without a real delta value
function writeBias(d, hasDelta = true) {
  const buf = Buffer.alloc(64)
  // Header: magic(8) version(8) n_curves(4) n_events(4) total_events(8) last_update(8)
  // = 40 bytes header. Data starts at offset 28 per training_pool.mjs logic.
  // We write a significant value at offset 28 to simulate a trained bias.
  if (hasDelta) {
    // Write 0.05 (> DELTA_THRESHOLD 0.005) as f64 LE at offset 28
    buf.writeBigUInt64LE(BigInt('0x3FA999999999999A'), 28)  // ≈ 0.05 as f64 LE
  }
  // Write vocal_efficacy = 0.9 at the last 8 bytes
  buf.writeBigUInt64LE(BigInt('0x3FECCCCCCCCCCCCD'), buf.length - 8)  // ≈ 0.9
  writeFileSync(join(d, 'learned_bias.bin'), buf)
}

describe('HarvestPipeline', () => {
  it('evaluate() passes a 60s session with bias delta', () => {
    const d = dir('pass-60')
    writeBias(d, true)
    const hp = new HarvestPipeline({ poolDir: dir('pool-pass-60'), mergeIntervalMs: 0 })
    const r = hp.evaluate(d, 0, 60_000)
    assert.ok(r.pass, `should pass 60s+delta: ${r.reason}`)
  })

  it('evaluate() fails a session shorter than 60s', () => {
    const d = dir('fail-short')
    writeBias(d, true)
    const hp = new HarvestPipeline({ poolDir: dir('pool-short'), mergeIntervalMs: 0 })
    const r = hp.evaluate(d, 0, 59_000)
    assert.equal(r.pass, false)
    assert.equal(r.reason, 'too_short')
  })

  it('evaluate() fails a 60s session with no bias delta', () => {
    const d = dir('fail-nodelta')
    writeBias(d, false)
    const hp = new HarvestPipeline({ poolDir: dir('pool-nodelta'), mergeIntervalMs: 0 })
    const r = hp.evaluate(d, 0, 90_000)
    assert.equal(r.pass, false)
  })

  it('evaluate() fails when learned_bias.bin is missing', () => {
    const d = dir('fail-nobias')
    // No bias file written
    const hp = new HarvestPipeline({ poolDir: dir('pool-nobias'), mergeIntervalMs: 0 })
    const r = hp.evaluate(d, 0, 90_000)
    assert.equal(r.pass, false)
  })

  it('evaluate() relaxed path: 60-300s session with delta → delta_only_relaxed_gate', () => {
    const d = dir('relaxed')
    writeBias(d, true)
    const hp = new HarvestPipeline({ poolDir: dir('pool-relaxed'), mergeIntervalMs: 0 })
    const r = hp.evaluate(d, 0, 120_000)
    assert.ok(r.pass)
    assert.equal(r.reason, 'delta_only_relaxed_gate')
  })

  it('collect() returns null when evaluate fails', () => {
    const hp = new HarvestPipeline({ poolDir: dir('pool-collect-fail'), mergeIntervalMs: 0 })
    const result = hp.collect('/nonexistent', 'sess1', 0, 30_000)
    assert.equal(result, null)
  })

  it('stats().poolSize = 0 for empty/missing pool dir', () => {
    const hp = new HarvestPipeline({ poolDir: join(BASE, 'nonexistent-pool'), mergeIntervalMs: 0 })
    assert.equal(hp.stats().poolSize, 0)
  })

  it('stats().poolSize reflects .bin files in pool dir', () => {
    const poolDir = dir('pool-size')
    const hp = new HarvestPipeline({ poolDir, mergeIntervalMs: 0 })
    assert.equal(hp.stats().poolSize, 0)
    writeFileSync(join(poolDir, 'a.bin'), Buffer.alloc(8))
    writeFileSync(join(poolDir, 'b.bin'), Buffer.alloc(8))
    assert.equal(hp.stats().poolSize, 2)
  })

  it('stats().totalMerges starts at 0', () => {
    const hp = new HarvestPipeline({ poolDir: dir('pool-merges'), mergeIntervalMs: 0 })
    assert.equal(hp.stats().totalMerges, 0)
  })

  it('mergeNow() returns {merged:0} when no primeMgr', async () => {
    const hp = new HarvestPipeline({ poolDir: dir('pool-noprime'), mergeIntervalMs: 0 })
    const r = await hp.mergeNow()
    assert.equal(r.merged, 0)
  })

  it('stopAutoMerge() is idempotent', () => {
    const hp = new HarvestPipeline({ poolDir: dir('pool-stop'), mergeIntervalMs: 5000 })
    assert.doesNotThrow(() => hp.stopAutoMerge())
    assert.doesNotThrow(() => hp.stopAutoMerge())
  })

  it('stats().lastMergeAt is null initially', () => {
    const hp = new HarvestPipeline({ poolDir: dir('pool-lat'), mergeIntervalMs: 0 })
    assert.equal(hp.stats().lastMergeAt, null)
  })
})
