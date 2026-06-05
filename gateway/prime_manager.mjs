/**
 * prime_manager.mjs — manages the Prime body (master AI, 7×24 alive).
 *
 * The Prime body:
 *   - Lives in a dedicated runtimeDir (not /tmp — survives restarts)
 *   - Its learned_bias.bin IS the global master seed for all visitor sessions
 *   - Is fed a continuous vocal/sensory curriculum by nursery_feeder.mjs
 *   - Absorbs training pool merges → continuously grows
 */
import { spawn }          from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, copyFileSync } from 'node:fs'
import { join }           from 'node:path'

export class PrimeManager {
  #bodyBin
  #runtimeDir
  #masterBiasPath
  #clock
  #proc = null
  #startedAt = 0
  #restartCount = 0
  #lastRestartAt = 0
  #crashWindow = 300_000   // 5 min — reset crash count after quiet period
  #maxRestarts = 5
  #crashCount = 0
  #crashCallbacks = []
  #stopped = false

  constructor({ bodyBin, runtimeDir, masterBiasPath, clock } = {}) {
    this.#bodyBin = bodyBin
    this.#runtimeDir = runtimeDir
    this.#masterBiasPath = masterBiasPath
    this.#clock = clock ?? (() => globalThis.Date ? globalThis.Date.now() : 0)
  }

  async start() {
    if (this.#proc) return   // idempotent
    mkdirSync(this.#runtimeDir, { recursive: true })
    // Seed from master bias if runtime doesn't have one yet
    const biasInRuntime = join(this.#runtimeDir, 'learned_bias.bin')
    if (!existsSync(biasInRuntime) && this.#masterBiasPath && existsSync(this.#masterBiasPath)) {
      try { copyFileSync(this.#masterBiasPath, biasInRuntime) } catch {}
    }
    this.#spawn()
  }

  stop() {
    this.#stopped = true
    if (this.#proc) { this.#proc.kill('SIGTERM'); this.#proc = null }
  }

  isAlive() { return !!this.#proc && !this.#stopped }

  restartCount() { return this.#restartCount }

  onCrash(cb) { this.#crashCallbacks.push(cb) }

  getMasterBiasPath() { return join(this.#runtimeDir, 'learned_bias.bin') }

  getRuntimeDir() { return this.#runtimeDir }

  getSocketPath() { return join(this.#runtimeDir, 'net.sock') }

  getStatus() {
    const now = this.#clock()
    const result = {
      alive:        this.isAlive(),
      reserve:      null,
      vitalState:   null,
      nPrototypes:  null,
      vocalEfficacy: null,
      restartCount: this.#restartCount,
      uptimeSec:    this.#startedAt > 0 ? Math.floor((now - this.#startedAt) / 1000) : 0,
      mergeCount:   0,
    }
    try {
      const vc = readFileSync(join(this.#runtimeDir, 'vital_core.bin'))
      if (vc.length >= 41) {
        const ab = vc.buffer.slice(vc.byteOffset, vc.byteOffset + vc.byteLength)
        const dv = new DataView(ab)
        result.reserve    = parseFloat(dv.getFloat64(16, true).toFixed(3))
        result.vitalState = vc[40] === 0 ? 'Alive' : 'Standby'
      }
    } catch {}
    try {
      const lb = readFileSync(join(this.#runtimeDir, 'learned_bias.bin'))
      if (lb.length >= 8) {
        const ab = lb.buffer.slice(lb.byteOffset, lb.byteOffset + lb.byteLength)
        const dv = new DataView(ab)
        result.vocalEfficacy = parseFloat(dv.getFloat64(lb.length - 8, true).toFixed(5))
      }
    } catch {}
    return result
  }

  #spawn() {
    if (this.#stopped) return
    const socketPath = join(this.#runtimeDir, 'net.sock')
    const proc = spawn(this.#bodyBin, [this.#runtimeDir, '--headless', socketPath], {
      stdio: 'ignore', detached: false,
    })
    this.#proc = proc
    this.#startedAt = this.#clock()

    proc.on('exit', (code) => {
      if (this.#stopped) return
      this.#proc = null
      const now = this.#clock()
      if (now - this.#lastRestartAt > this.#crashWindow) this.#crashCount = 0
      this.#crashCount++
      this.#lastRestartAt = now

      if (this.#crashCount > this.#maxRestarts) {
        for (const cb of this.#crashCallbacks) cb(code, this.#restartCount)
        return  // give up
      }
      this.#restartCount++
      const delay = Math.min(5000 * this.#crashCount, 30_000)
      setTimeout(() => this.#spawn(), delay)
    })
  }
}
