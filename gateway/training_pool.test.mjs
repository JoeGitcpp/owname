/**
 * training_pool.test.mjs
 * Run: node --test gateway/training_pool.test.mjs
 *
 * Covers the familiar_objects.bin binary layout (regression for the 52→84
 * stride bug in Gate 2) and the evaluate_session gates end-to-end.
 */
import { describe, it, after } from 'node:test'
import assert from 'node:assert/strict'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { parseFamiliarObjects, evaluate_session } from './training_pool.mjs'

const BASE = join(tmpdir(), `training-pool-test-${process.pid}`)
after(() => { try { rmSync(BASE, { recursive: true, force: true }) } catch {} })

// ── fixture builders (mirror the Rust serializers) ───────────────────────────

// familiar_objects.bin: header 20B + N×84B objects + 8B trailer.
// object: fingerprint 8×f64 (64) + familiarity f64 (8) + recognition_count u32 (4)
//         + last_seen_us u64 (8) = 84
function buildFamiliarObjects(objs) {
  const header = Buffer.alloc(20)
  header.writeBigUInt64LE(0xB10B10000000C5n, 0)   // MAGIC
  header.writeBigUInt64LE(2n, 8)                    // VERSION
  header.writeUInt32LE(objs.length, 16)             // count
  const body = Buffer.concat(objs.map(o => {
    const b = Buffer.alloc(84)
    const fp = o.fingerprint ?? []
    for (let i = 0; i < 8; i++) b.writeDoubleLE(fp[i] ?? 0, i * 8)
    b.writeDoubleLE(o.familiarity ?? 0, 64)
    b.writeUInt32LE(o.recognitionCount ?? 0, 72)
    b.writeBigUInt64LE(BigInt(o.lastSeenUs ?? 0), 76)
    return b
  }))
  const trailer = Buffer.alloc(8)                   // last_update_t_us
  return Buffer.concat([header, body, trailer])
}

// vital_core.bin: reserve f64 @16, vstate u8 @40. Make it "alive".
function buildVitalCore({ reserve = 1, vstate = 0 } = {}) {
  const b = Buffer.alloc(57)
  b.writeDoubleLE(reserve, 16)
  b[40] = vstate
  return b
}

// learned_bias.bin (v5): a real delta f64 after the header (@28), and the v5
// TAIL [..., vocal_efficacy, developmental_maturity] — efficacy at length-16,
// maturity at length-8. (Putting efficacy ONLY in the tail makes the healthy
// case a regression test for the Gate-5 offset fix: the old getFloat64(20) read
// would see 0 here and wrongly reject as low_vocal_eff.)
function buildLearnedBias({ vocalEff = 0.5, delta = 0.1 } = {}) {
  const b = Buffer.alloc(64)
  b.writeDoubleLE(delta, 28)
  b.writeDoubleLE(vocalEff, b.length - 16)
  b.writeDoubleLE(0, b.length - 8)
  return b
}

function makeSession(name, { fam, vital, bias }) {
  const dir = join(BASE, name)
  mkdirSync(dir, { recursive: true })
  if (fam)   writeFileSync(join(dir, 'familiar_objects.bin'), fam)
  if (vital) writeFileSync(join(dir, 'vital_core.bin'), vital)
  if (bias)  writeFileSync(join(dir, 'learned_bias.bin'), bias)
  return dir
}

// ── parseFamiliarObjects (the stride-bug regression) ─────────────────────────

describe('parseFamiliarObjects (84-byte stride)', () => {
  it('reads count, familiarity and recognition_count correctly', () => {
    const buf = buildFamiliarObjects([
      { familiarity: 0.2, recognitionCount: 7 },
      { familiarity: 0.9, recognitionCount: 250 },   // most-familiar
      { familiarity: 0.5, recognitionCount: 40 },
    ])
    // total = 20 + 3*84 + 8 = 280
    assert.equal(buf.length, 280)
    const p = parseFamiliarObjects(buf)
    assert.equal(p.count, 3)
    // topRecog must come from the MOST-FAMILIAR object (0.9 → 250), which the
    // old 52-byte stride could never read correctly.
    assert.equal(p.topFam, 0.9)
    assert.equal(p.topRecog, 250)
  })

  it('tolerates an empty / too-short buffer', () => {
    assert.deepEqual(parseFamiliarObjects(Buffer.alloc(0)), { count: 0, topFam: 0, topRecog: 0, objects: [] })
  })
})

// ── evaluate_session gates ───────────────────────────────────────────────────

describe('evaluate_session', () => {
  const LONG = { startedAt: 0, endedAt: 6 * 60_000 }   // 6 min → past Gate 1

  it('passes a healthy session (recog ≥ 20, alive, delta, vocal_eff)', () => {
    const dir = makeSession('good', {
      fam:   buildFamiliarObjects([{ familiarity: 0.8, recognitionCount: 25 }]),
      vital: buildVitalCore({ reserve: 1, vstate: 0 }),
      bias:  buildLearnedBias({ vocalEff: 0.5, delta: 0.1 }),
    })
    const r = evaluate_session({ runtimeDir: dir, ...LONG })
    assert.equal(r.pass, true, `expected pass, got ${r.reason}`)
  })

  it('rejects low recognition count (Gate 2)', () => {
    const dir = makeSession('lowrecog', {
      fam:   buildFamiliarObjects([{ familiarity: 0.8, recognitionCount: 5 }]),
      vital: buildVitalCore({ reserve: 1, vstate: 0 }),
      bias:  buildLearnedBias({ vocalEff: 0.5, delta: 0.1 }),
    })
    const r = evaluate_session({ runtimeDir: dir, ...LONG })
    assert.equal(r.pass, false)
    assert.match(r.reason, /low_recog:5/)
  })

  it('salvages an engaged session that ended in Standby (recognition proves it was alive)', () => {
    // Regression: a talky call drains reserve to the floor and ends in Standby,
    // but it recognised 25 objects → it was alive and engaged. Must NOT be
    // discarded as standby_only.
    const dir = makeSession('engaged-standby', {
      fam:   buildFamiliarObjects([{ familiarity: 0.8, recognitionCount: 25 }]),
      vital: buildVitalCore({ reserve: 0, vstate: 1 }),
      bias:  buildLearnedBias({ vocalEff: 0.5, delta: 0.1 }),
    })
    const r = evaluate_session({ runtimeDir: dir, ...LONG })
    assert.equal(r.pass, true, `expected pass (engaged-but-tired), got ${r.reason}`)
  })

  it('rejects low vocal efficacy read from the v5 tail (Gate 5)', () => {
    const dir = makeSession('lowvocal', {
      fam:   buildFamiliarObjects([{ familiarity: 0.8, recognitionCount: 25 }]),
      vital: buildVitalCore({ reserve: 1, vstate: 0 }),
      bias:  buildLearnedBias({ vocalEff: 0.005, delta: 0.1 }), // below VOCAL_EFF_MIN
    })
    const r = evaluate_session({ runtimeDir: dir, ...LONG })
    assert.equal(r.pass, false)
    assert.match(r.reason, /low_vocal_eff/)
  })
})
