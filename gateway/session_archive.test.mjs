/**
 * session_archive.test.mjs
 * Run: node --test gateway/session_archive.test.mjs
 */
import { describe, it, after } from 'node:test'
import assert from 'node:assert/strict'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  mkdirSync, writeFileSync, rmSync, existsSync, readFileSync, utimesSync,
} from 'node:fs'
import { archiveSessionDiag, pruneSessionArchive } from './session_archive.mjs'

const BASE = join(tmpdir(), `session-archive-test-${process.pid}`)
const RUNTIME = join(BASE, 'runtime')
const ARCHIVE = join(BASE, 'archive')
after(() => { try { rmSync(BASE, { recursive: true, force: true }) } catch {} })

describe('archiveSessionDiag', () => {
  it('copies vocal_diag.csv and writes meta.json before cleanup', () => {
    mkdirSync(RUNTIME, { recursive: true })
    writeFileSync(join(RUNTIME, 'vocal_diag.csv'), 'ts,eff\n1,0.2\n')
    // a learning binary that must NOT be archived
    writeFileSync(join(RUNTIME, 'snapshot.bin'), Buffer.alloc(16))

    const archived = archiveSessionDiag(RUNTIME, ARCHIVE, 'sid123', { mode: 'ephemeral' })
    assert.deepEqual(archived, ['vocal_diag.csv'])

    const dst = join(ARCHIVE, 'sid123')
    assert.ok(existsSync(join(dst, 'vocal_diag.csv')), 'vocal_diag.csv archived')
    assert.ok(!existsSync(join(dst, 'snapshot.bin')), 'snapshot.bin must NOT be archived')

    const meta = JSON.parse(readFileSync(join(dst, 'meta.json'), 'utf8'))
    assert.equal(meta.sessionId, 'sid123')
    assert.equal(meta.mode, 'ephemeral')
    assert.deepEqual(meta.archived, ['vocal_diag.csv'])
  })

  it('still writes meta.json when no diag files exist', () => {
    const rt = join(BASE, 'empty-runtime')
    mkdirSync(rt, { recursive: true })
    const archived = archiveSessionDiag(rt, ARCHIVE, 'sidEmpty', { reason: 'standby_only' })
    assert.deepEqual(archived, [])
    const meta = JSON.parse(readFileSync(join(ARCHIVE, 'sidEmpty', 'meta.json'), 'utf8'))
    assert.equal(meta.reason, 'standby_only')
  })
})

describe('pruneSessionArchive', () => {
  it('removes session dirs older than maxAgeDays, keeps fresh ones', () => {
    const old = join(ARCHIVE, 'old'); mkdirSync(old, { recursive: true })
    writeFileSync(join(old, 'meta.json'), '{}')
    const fresh = join(ARCHIVE, 'fresh'); mkdirSync(fresh, { recursive: true })
    writeFileSync(join(fresh, 'meta.json'), '{}')
    // backdate `old` to 30 days ago
    const longAgo = Date.now() / 1000 - 30 * 86400
    utimesSync(old, longAgo, longAgo)

    const pruned = pruneSessionArchive(ARCHIVE, { maxAgeDays: 14 })
    assert.ok(pruned >= 1)
    assert.ok(!existsSync(old), 'old session pruned')
    assert.ok(existsSync(fresh), 'fresh session kept')
  })
})
