import { randomBytes } from 'node:crypto'

export class InstanceRegistry {
  #instances = new Map()  // id → data
  #byIdentity = new Map() // identityKey → id

  create(identityKey) {
    if (this.#byIdentity.has(identityKey)) {
      throw new Error(`instance already exists for identity: ${identityKey}`)
    }
    const id   = randomBytes(16).toString('hex')
    const data = { id, identityKey, state: 'pending_verification', createdAt: Date.now() }
    this.#instances.set(id, data)
    this.#byIdentity.set(identityKey, id)
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
  }

  setState(id, state) {
    const d = this.#instances.get(id)
    if (!d) throw new Error(`instance not found: ${id}`)
    d.state = state
  }

  count() {
    return this.#instances.size
  }
}
