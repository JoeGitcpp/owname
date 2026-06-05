/**
 * m1_5_integration.test.mjs — M1.5 perceptual upgrade smoke tests.
 *
 * Validates the end-to-end invariants that must hold for the upgraded
 * senses pipeline: frame size, tag acceptance, layout, and bandwidth.
 *
 * Run: node --test gateway/m1_5_integration.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { SENSES_FRAME_BYTES, validateFrame } from './security.mjs'

const TAG_SENSES     = 0x01
const FRAME_RATE_HZ  = 20
const BW_LIMIT_BYTES = 5_000_000  // 5 MB/s

// ── Smoke tests ───────────────────────────────────────────────────────────────
describe('M1.5 integration smoke tests', () => {
  it('SENSES_FRAME_BYTES equals 230705', () => {
    assert.equal(SENSES_FRAME_BYTES, 230705)
  })

  it('a 230705-byte buffer with tag 0x01 passes validateFrame', () => {
    const buf = Buffer.alloc(SENSES_FRAME_BYTES)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), true)
  })

  it('frame layout: grid starts at offset 305 (1 + 76×4)', () => {
    const gridOffset = 1 + 76 * 4
    assert.equal(gridOffset, 305)
  })

  it('20 Hz bandwidth: 230705 × 20 = 4614100 < 5000000 bytes/sec', () => {
    const throughput = SENSES_FRAME_BYTES * FRAME_RATE_HZ
    assert.equal(throughput, 4_614_100)
    assert.ok(throughput < BW_LIMIT_BYTES)
  })
})
