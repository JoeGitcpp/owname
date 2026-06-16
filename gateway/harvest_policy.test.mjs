/**
 * harvest_policy.test.mjs
 * Run: node --test gateway/harvest_policy.test.mjs
 */
import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import { decideHarvest, emaUpdate } from './harvest_policy.mjs'

const S = 1000
const HARVEST_S = 120, HARVEST_MAX_S = 600, MIN_GAIN = 0.005

describe('emaUpdate', () => {
  it('seeds from the first sample', () => {
    assert.equal(emaUpdate(null, 0.23), 0.23)
    assert.equal(emaUpdate(NaN, 0.23), 0.23)
  })
  it('a single spike cannot dominate the baseline', () => {
    // prev ~0.23, one tick spikes to 0.43 → EMA stays well below 0.43.
    const next = emaUpdate(0.23, 0.43, 0.2)
    assert.ok(next < 0.30, `expected smoothed value < 0.30, got ${next}`)
  })
})

describe('decideHarvest', () => {
  const base = {
    now: 1_000_000, lastCheck: 1_000_000 - HARVEST_S * S, lastHarvest: 1_000_000,
    emaEff: 0.23, lastHarvestEff: 0.23, harvestS: HARVEST_S, harvestMaxS: HARVEST_MAX_S, minGain: MIN_GAIN,
  }

  it('throttles: within the check window → no check, no harvest', () => {
    const d = decideHarvest({ ...base, lastCheck: base.now - 10 * S })
    assert.equal(d.check, false)
    assert.equal(d.harvest, false)
  })

  it('harvests on genuine improvement (smoothed efficacy rose)', () => {
    const d = decideHarvest({ ...base, emaEff: 0.30, lastHarvestEff: 0.23 })
    assert.equal(d.check, true)
    assert.equal(d.improved, true)
    assert.equal(d.harvest, true)
  })

  it('flat efficacy within HARVEST_MAX_S → check but no harvest', () => {
    const d = decideHarvest({ ...base, emaEff: 0.23, lastHarvestEff: 0.23, lastHarvest: base.now - 200 * S })
    assert.equal(d.check, true)
    assert.equal(d.improved, false)
    assert.equal(d.forced, false)
    assert.equal(d.harvest, false)
  })

  it('forced harvest fires after HARVEST_MAX_S even with no improvement (regression for the dead safety valve)', () => {
    // The OLD code reset lastHarvest every 120s check, so this state was never
    // reachable. With the fix, lastHarvest only advances on an actual harvest.
    const d = decideHarvest({
      ...base, emaEff: 0.23, lastHarvestEff: 0.42936,        // baseline still poisoned-high
      lastHarvest: base.now - HARVEST_MAX_S * S,             // 600s since last actual harvest
    })
    assert.equal(d.check, true)
    assert.equal(d.improved, false)
    assert.equal(d.forced, true)
    assert.equal(d.harvest, true)
  })
})
