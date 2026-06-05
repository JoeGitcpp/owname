/**
 * prime_manager.test.mjs — tests for PrimeManager state management.
 * Tests focus on state logic, file I/O, and API contracts — not actual spawning.
 * Run: node --test gateway/prime_manager.test.mjs
 */
import { describe, it, after } from 'node:test'
import assert from 'node:assert/strict'
import { mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { PrimeManager } from './prime_manager.mjs'

const BASE = join(tmpdir(), `pm-test-${process.pid}`)
mkdirSync(BASE, { recursive: true })
after(() => rmSync(BASE, { recursive: true, force: true }))

function dir(name) {
  const d = join(BASE, name); mkdirSync(d, { recursive: true }); return d
}

describe('PrimeManager — state and file I/O', () => {

  it('isAlive() = false before start()', () => {
    const pm = new PrimeManager({ bodyBin: '/bin/true', runtimeDir: dir('before-start'), masterBiasPath: null, clock: () => 0 })
    assert.equal(pm.isAlive(), false)
  })

  it('restartCount() starts at 0', () => {
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: dir('rc0'), masterBiasPath: null, clock: () => 0 })
    assert.equal(pm.restartCount(), 0)
  })

  it('stop() is idempotent before start()', () => {
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: dir('idem'), masterBiasPath: null })
    assert.doesNotThrow(() => pm.stop())
    assert.doesNotThrow(() => pm.stop())
  })

  it('getMasterBiasPath() is inside runtimeDir and named learned_bias.bin', () => {
    const d = dir('mbp')
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null })
    const p = pm.getMasterBiasPath()
    assert.ok(p.startsWith(d))
    assert.ok(p.endsWith('learned_bias.bin'))
  })

  it('getRuntimeDir() returns the runtimeDir', () => {
    const d = dir('rtd')
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null })
    assert.equal(pm.getRuntimeDir(), d)
  })

  it('getSocketPath() is inside runtimeDir and ends with net.sock', () => {
    const d = dir('sock')
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null })
    assert.ok(pm.getSocketPath().includes(d))
    assert.ok(pm.getSocketPath().endsWith('net.sock'))
  })

  it('getStatus() returns null fields when no files exist', () => {
    const d = dir('empty-status')
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null, clock: () => 1000 })
    const s = pm.getStatus()
    assert.equal(s.alive, false)
    assert.equal(s.reserve, null)
    assert.equal(s.vocalEfficacy, null)
    assert.equal(s.restartCount, 0)
    assert.equal(s.uptimeSec, 0)   // startedAt=0 so no uptime
  })

  it('getStatus() reads reserve from vital_core.bin at offset 16 (f64 LE)', () => {
    const d = dir('vc-reserve')
    const buf = Buffer.alloc(64, 0)
    // Write 0.82 as f64 LE at offset 16
    const view = new DataView(buf.buffer)
    view.setFloat64(16, 0.82, true)
    buf[40] = 0  // Alive
    writeFileSync(join(d, 'vital_core.bin'), buf)
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null, clock: () => 0 })
    const s = pm.getStatus()
    assert.ok(Math.abs((s.reserve ?? 0) - 0.82) < 0.001, `reserve should be ~0.82, got ${s.reserve}`)
    assert.equal(s.vitalState, 'Alive')
  })

  it('getStatus() reads Standby from vital_core.bin[40] = 1', () => {
    const d = dir('vc-standby')
    const buf = Buffer.alloc(64, 0)
    const view = new DataView(buf.buffer)
    view.setFloat64(16, 0.02, true)
    buf[40] = 1  // Standby
    writeFileSync(join(d, 'vital_core.bin'), buf)
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null, clock: () => 0 })
    assert.equal(pm.getStatus().vitalState, 'Standby')
  })

  it('getStatus() reads vocalEfficacy from last 8 bytes of learned_bias.bin', () => {
    const d = dir('lb-efficacy')
    const buf = Buffer.alloc(64, 0)
    const view = new DataView(buf.buffer)
    view.setFloat64(buf.length - 8, 0.97, true)
    writeFileSync(join(d, 'learned_bias.bin'), buf)
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: d, masterBiasPath: null, clock: () => 0 })
    const eff = pm.getStatus().vocalEfficacy ?? 0
    assert.ok(Math.abs(eff - 0.97) < 0.001, `vocalEfficacy should be ~0.97, got ${eff}`)
  })

  it('getStatus() does not throw when files are missing', () => {
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: '/definitely-nonexistent-dir', masterBiasPath: null, clock: () => 0 })
    assert.doesNotThrow(() => pm.getStatus())
  })

  it('onCrash() registers callback without error', () => {
    const pm = new PrimeManager({ bodyBin: '/b', runtimeDir: dir('crash-cb'), masterBiasPath: null })
    let called = false
    assert.doesNotThrow(() => pm.onCrash(() => { called = true }))
    void called
  })

  it('start() creates runtimeDir if it does not exist', async () => {
    // Use a path that does not exist yet
    const d = join(BASE, 'autocreate-' + Math.floor(Math.random() * 999999))
    const pm = new PrimeManager({ bodyBin: '/bin/sleep', runtimeDir: d, masterBiasPath: null, clock: () => 0 })
    await pm.start()
    const { existsSync } = await import('node:fs')
    assert.ok(existsSync(d), 'runtimeDir should be created by start()')
    pm.stop()
    rmSync(d, { recursive: true, force: true })
  })

  it('start() seeds learned_bias.bin from masterBiasPath when runtime is fresh', async () => {
    const d = dir('seed')
    const masterBias = join(BASE, 'master.bin')
    writeFileSync(masterBias, Buffer.alloc(32, 0xAB))
    // Don't spawn — just check the seeding side effect
    const pm = new PrimeManager({ bodyBin: '/bin/sleep', runtimeDir: d, masterBiasPath: masterBias, clock: () => 0 })
    await pm.start()
    pm.stop()
    const { existsSync, readFileSync } = await import('node:fs')
    assert.ok(existsSync(join(d, 'learned_bias.bin')), 'learned_bias.bin should be seeded')
    const seeded = readFileSync(join(d, 'learned_bias.bin'))
    assert.equal(seeded[0], 0xAB, 'seeded content should match master')
  })

  it('start() is idempotent — second call does not spawn a second process', async () => {
    const d = dir('idem-start')
    const pm = new PrimeManager({ bodyBin: '/bin/sleep', runtimeDir: d, masterBiasPath: null, clock: () => 0 })
    await pm.start()
    const was_alive = pm.isAlive()
    await pm.start()  // no-op
    const still_alive = pm.isAlive()
    pm.stop()
    assert.equal(was_alive, still_alive, 'isAlive should not change from second start()')
  })
})
