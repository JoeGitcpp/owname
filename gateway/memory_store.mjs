import { mkdirSync, writeFileSync, readFileSync, existsSync, rmSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { createHash } from 'node:crypto'

// Hash the identity key into the on-disk namespace so raw email/phone never
// lands in a directory name (audit: the binary-checkpoint dir already does this;
// the KV store didn't). Matches server.mjs's userKey derivation pattern.
function nsDir(base, identityKey) {
  const h = createHash('sha256').update(String(identityKey)).digest('hex').slice(0, 32)
  return join(base, h)
}

function keyFile(dir, key) {
  return join(dir, encodeURIComponent(key) + '.json')
}

export class MemoryStore {
  #base

  constructor(basePath) {
    this.#base = basePath
    mkdirSync(basePath, { recursive: true })
  }

  write(identityKey, key, value) {
    const dir = nsDir(this.#base, identityKey)
    mkdirSync(dir, { recursive: true })
    writeFileSync(keyFile(dir, key), JSON.stringify(value), 'utf8')
  }

  read(identityKey, key) {
    const path = keyFile(nsDir(this.#base, identityKey), key)
    if (!existsSync(path)) return null
    return JSON.parse(readFileSync(path, 'utf8'))
  }

  listKeys(identityKey) {
    const dir = nsDir(this.#base, identityKey)
    if (!existsSync(dir)) return []
    return readdirSync(dir)
      .filter(f => f.endsWith('.json'))
      .map(f => decodeURIComponent(f.slice(0, -5)))
  }

  deleteNamespace(identityKey) {
    const dir = nsDir(this.#base, identityKey)
    if (existsSync(dir)) rmSync(dir, { recursive: true, force: true })
  }

  harvest(identityKey) {
    const result = {}
    for (const key of this.listKeys(identityKey)) {
      result[key] = this.read(identityKey, key)
    }
    return result
  }
}
