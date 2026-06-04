import { mkdirSync, writeFileSync, readFileSync, existsSync, rmSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

function nsDir(base, identityKey) {
  return join(base, encodeURIComponent(identityKey))
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
