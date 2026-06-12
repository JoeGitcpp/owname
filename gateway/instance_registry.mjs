import { randomBytes } from 'node:crypto'
import { writeFileSync, readFileSync, existsSync, renameSync, mkdirSync } from 'node:fs'
import { dirname } from 'node:path'

export class InstanceRegistry {
  #instances = new Map()  // id → data
  #byIdentity = new Map() // identityKey → id
  #persistPath = null

  // persistPath: optional JSON file. When set, the registry is loaded from it on
  // construction and atomically re-written on every mutation, so a returning
  // user's persistent instance survives a gateway restart (the in-memory-only
  // registry lost all instances on restart — the core "long-term instance" bug).
  constructor(persistPath = null) {
    this.#persistPath = persistPath
    if (persistPath && existsSync(persistPath)) {
      try {
        const arr = JSON.parse(readFileSync(persistPath, 'utf8'))
        for (const d of Array.isArray(arr) ? arr : []) {
          if (d && typeof d.id === 'string' && typeof d.identityKey === 'string') {
            this.#instances.set(d.id, d)
            this.#byIdentity.set(d.identityKey, d.id)
          }
        }
      } catch { /* corrupt persisted registry → start empty */ }
    }
  }

  #save() {
    if (!this.#persistPath) return
    try {
      mkdirSync(dirname(this.#persistPath), { recursive: true })
      const tmp = this.#persistPath + '.tmp'
      writeFileSync(tmp, JSON.stringify([...this.#instances.values()]), 'utf8')
      renameSync(tmp, this.#persistPath) // atomic — no half-written registry
    } catch { /* best-effort persistence */ }
  }

  create(identityKey, name = null) {
    if (this.#byIdentity.has(identityKey)) {
      throw new Error(`instance already exists for identity: ${identityKey}`)
    }
    const id   = randomBytes(16).toString('hex')
    const data = {
      id,
      identityKey,
      name: name || null,
      state: 'pending_verification',
      createdAt: Date.now(),
      lastSeenAt: Date.now(),
    }
    this.#instances.set(id, data)
    this.#byIdentity.set(identityKey, id)
    this.#save()
    return { ...data }
  }

  get(id) {
    const d = this.#instances.get(id)
    return d ? { ...d } : null
  }

  getByIdentity(identityKey) {
    const id = this.#byIdentity.get(identityKey)
    return id ? this.get(id) : null
  }

  list() {
    return [...this.#instances.values()].map(d => ({ ...d }))
  }

  destroy(id) {
    const d = this.#instances.get(id)
    if (!d) return
    this.#byIdentity.delete(d.identityKey)
    this.#instances.delete(id)
    this.#save()
  }

  setState(id, state) {
    const d = this.#instances.get(id)
    if (!d) throw new Error(`instance not found: ${id}`)
    d.state = state
    this.#save()
  }

  setName(id, name) {
    const d = this.#instances.get(id)
    if (!d) throw new Error(`instance not found: ${id}`)
    d.name = name || null
    this.#save()
  }

  // Record that the instance was just connected to — for "last seen" display and
  // for reattachment bookkeeping across restarts.
  touch(id) {
    const d = this.#instances.get(id)
    if (!d) return
    d.lastSeenAt = Date.now()
    this.#save()
  }

  count() {
    return this.#instances.size
  }
}
