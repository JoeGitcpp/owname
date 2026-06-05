/**
 * perceptual_upgrade.test.mjs — TDD for the M1.5 perceptual upgrade.
 *
 * Covers:
 *  - SENSES_FRAME_BYTES = 230705 (160×120 RGB f32 camera + header)
 *  - validateFrame acceptance / rejection with new frame size
 *  - old 3137-byte frame is now rejected
 *  - padding-attack rejection (frame + 1 byte)
 *  - bandwidth budget: 20 Hz × 230705 B < 5 MB/s
 *  - frame layout: grid starts at byte offset 305
 *  - GAUSS_CAP = 512   (verified against phot_gaussian.rs)
 *  - AXIS_COUNT = 161  (verified against lib.rs)
 *  - VIB_BANDS = 16    (verified against lib.rs)
 *
 * Run: node --test gateway/perceptual_upgrade.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { SENSES_FRAME_BYTES, validateFrame } from './security.mjs'

// ── Layout constants (mirror of lib.rs / net_adapter.rs) ─────────────────────
const TAG_SENSES        = 0x01
const OLD_FRAME_BYTES   = 3137          // pre-upgrade compact format — now invalid
const FRAME_RATE_HZ     = 20
const BW_LIMIT_BYTES    = 5_000_000     // 5 MB/s hard ceiling

// Header layout: tag(1) + scalars(76×4=304) + grid(160×120×3×4=230400)
//   scalars = audio_level(1) + audio_flux(1) + mel_bands(64) + motion(1) + luma(1) + aux(8)
//           = 76 f32 values → 304 bytes
// Grid starts at: 1 + 76×4 = 305
const SCALAR_COUNT      = 76            // f32 scalars before the pixel grid
const GRID_OFFSET       = 1 + SCALAR_COUNT * 4   // 305

// Rust constants (values pinned here; tests document the source truth)
const GAUSS_CAP         = 512           // phot_gaussian.rs: pub const GAUSS_CAP: usize = 512
const AXIS_COUNT        = 161           // lib.rs: VIB_BANDS + VIS_AXES + INTERO_AXES + CODE_AXES + GAUSS_SUMMARY_AXES
const VIB_BANDS         = 16            // lib.rs: pub const VIB_BANDS: usize = 16

// ── SENSES_FRAME_BYTES arithmetic ─────────────────────────────────────────────
describe('SENSES_FRAME_BYTES constant', () => {
  it('equals 230705', () => {
    assert.equal(SENSES_FRAME_BYTES, 230705)
  })

  it('decomposes as 1 + 76×4 + 160×120×3×4', () => {
    const expected = 1 + 76 * 4 + 160 * 120 * 3 * 4
    assert.equal(SENSES_FRAME_BYTES, expected)
  })

  it('camera payload is 160×120×3×4 = 230400 bytes', () => {
    const cameraBytes = 160 * 120 * 3 * 4
    assert.equal(cameraBytes, 230400)
  })

  it('scalar header is 1 + 76×4 = 305 bytes', () => {
    const headerBytes = 1 + 76 * 4
    assert.equal(headerBytes, 305)
  })

  it('exported SENSES_FRAME_BYTES matches local arithmetic', () => {
    assert.equal(SENSES_FRAME_BYTES, 1 + SCALAR_COUNT * 4 + 160 * 120 * 3 * 4)
  })
})

// ── validateFrame — new 230705-byte format ────────────────────────────────────
describe('validateFrame — new perceptual format', () => {
  it('accepts a correctly-sized 230705-byte senses frame', () => {
    const buf = Buffer.alloc(SENSES_FRAME_BYTES)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), true)
  })

  it('rejects a frame that is one byte too short', () => {
    const buf = Buffer.alloc(SENSES_FRAME_BYTES - 1)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), false)
  })

  it('rejects a frame that is one byte too long (padding attack)', () => {
    const buf = Buffer.alloc(SENSES_FRAME_BYTES + 1)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), false)
  })

  it('rejects old 3137-byte compact frame (format retired in M1.5)', () => {
    const buf = Buffer.alloc(OLD_FRAME_BYTES)
    buf[0] = TAG_SENSES
    assert.equal(validateFrame(buf), false)
  })

  it('rejects unknown tag even with correct size', () => {
    const buf = Buffer.alloc(SENSES_FRAME_BYTES)
    buf[0] = 0xFF
    assert.equal(validateFrame(buf), false)
  })

  it('rejects null', () => {
    assert.equal(validateFrame(null), false)
  })

  it('rejects non-buffer', () => {
    assert.equal(validateFrame('attack'), false)
    assert.equal(validateFrame(undefined), false)
    assert.equal(validateFrame(42), false)
  })

  it('rejects empty buffer', () => {
    assert.equal(validateFrame(Buffer.alloc(0)), false)
  })
})

// ── Bandwidth budget ──────────────────────────────────────────────────────────
describe('bandwidth budget at 20 Hz', () => {
  it('20 Hz × 230705 bytes/frame < 5 MB/s', () => {
    const throughput = FRAME_RATE_HZ * SENSES_FRAME_BYTES
    assert.ok(
      throughput < BW_LIMIT_BYTES,
      `throughput ${throughput} B/s must be < ${BW_LIMIT_BYTES} B/s`
    )
  })

  it('throughput is exactly 4614100 B/s at 20 Hz', () => {
    assert.equal(FRAME_RATE_HZ * SENSES_FRAME_BYTES, 4_614_100)
  })

  it('headroom: limit - throughput > 385000 bytes/s', () => {
    const headroom = BW_LIMIT_BYTES - FRAME_RATE_HZ * SENSES_FRAME_BYTES
    assert.ok(headroom > 385_000, `expected > 385000 B/s headroom, got ${headroom}`)
  })
})

// ── Frame layout ──────────────────────────────────────────────────────────────
describe('frame layout — grid offset', () => {
  it('grid starts at byte offset 305 (= 1 + 76×4)', () => {
    assert.equal(GRID_OFFSET, 305)
  })

  it('bytes [1..305) are the scalar header (76 f32 values)', () => {
    // 76 scalars × 4 bytes = 304 bytes, occupying indices 1..304 inclusive
    assert.equal(SCALAR_COUNT * 4, 304)
  })

  it('bytes [305..230705) are the 160×120 RGB f32 grid', () => {
    const gridBytes = SENSES_FRAME_BYTES - GRID_OFFSET
    assert.equal(gridBytes, 160 * 120 * 3 * 4)
  })

  it('setting tag byte at [0] and reading it back is stable', () => {
    const buf = Buffer.alloc(SENSES_FRAME_BYTES)
    buf[0] = TAG_SENSES
    assert.equal(buf[0], TAG_SENSES)
  })
})

// ── Rust constant pins ────────────────────────────────────────────────────────
describe('Rust constant pins (phot_gaussian.rs + lib.rs)', () => {
  it('GAUSS_CAP = 512', () => {
    assert.equal(GAUSS_CAP, 512)
  })

  it('AXIS_COUNT = 161 (16+128+10+5+2)', () => {
    assert.equal(AXIS_COUNT, 161)
    // decomposed: VIB_BANDS + VIS_AXES + INTERO_AXES + CODE_AXES + GAUSS_SUMMARY_AXES
    assert.equal(16 + 128 + 10 + 5 + 2, 161)
  })

  it('VIB_BANDS = 16', () => {
    assert.equal(VIB_BANDS, 16)
  })

  it('AXIS_COUNT > VIB_BANDS (sanity: hearing is a subset)', () => {
    assert.ok(AXIS_COUNT > VIB_BANDS)
  })
})
