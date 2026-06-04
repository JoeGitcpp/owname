import { randomBytes } from 'node:crypto'

export class EphemeralSessionManager {
  #sessions = new Map()  // id → { ip, lastActive }
  #byIp     = new Map()  // ip → Set<id>
  #opts

  constructor({ perIpMax = 5, timeoutMs = 30 * 60_000, clock = Date.now } = {}) {
    this.#opts = { perIpMax, timeoutMs, clock }
  }

  create(ip) {
    this._sweep()
    const ipSet = this.#byIp.get(ip) ?? new Set()
    if (ipSet.size >= this.#opts.perIpMax) {
      throw new Error(`per-IP session limit reached for ${ip}`)
    }
    const id = randomBytes(16).toString('hex')
    this.#sessions.set(id, { ip, lastActive: this.#opts.clock() })
    ipSet.add(id)
    this.#byIp.set(ip, ipSet)
    return id
  }

  heartbeat(sessionId) {
    const s = this.#sessions.get(sessionId)
    if (s) s.lastActive = this.#opts.clock()
  }

  destroy(sessionId) {
    const s = this.#sessions.get(sessionId)
    if (!s) return
    this.#sessions.delete(sessionId)
    const ipSet = this.#byIp.get(s.ip)
    if (ipSet) {
      ipSet.delete(sessionId)
      if (ipSet.size === 0) this.#byIp.delete(s.ip)
    }
  }

  countForIp(ip) {
    this._sweep()
    return this.#byIp.get(ip)?.size ?? 0
  }

  size() {
    this._sweep()
    return this.#sessions.size
  }

  _sweep() {
    const now = this.#opts.clock()
    for (const [id, s] of this.#sessions) {
      if (now - s.lastActive >= this.#opts.timeoutMs) this.destroy(id)
    }
  }
}
