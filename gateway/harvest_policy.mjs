/**
 * harvest_policy.mjs — pure harvest-decision policy for the nursery feeder.
 *
 * Extracted from nursery_feeder.mjs so it can be unit-tested in isolation
 * (nursery_feeder.mjs auto-connects to the body socket on import).
 *
 * Fixes two prior bugs in the feeder's harvest gating:
 *
 *  1. forced-harvest never fired. The loop reset `lastHarvest = now` on every
 *     ~120s CHECK, before evaluating `forced = now - lastHarvest >= HARVEST_MAX_S`.
 *     So `now - lastHarvest` never grew past one check interval and the periodic
 *     "force a merge at least every 600s" safety valve was dead. Here the caller
 *     resets the THROTTLE clock (`lastCheck`) separately and only advances
 *     `lastHarvest` on an ACTUAL harvest, so the forced timer accumulates.
 *
 *  2. a transient warmup spike poisoned the baseline forever. The baseline was
 *     the instantaneous efficacy captured at the one harvest that landed (a
 *     t=90s spike of 0.42936); real efficacy then settled to ~0.23, so
 *     `improved` could never be true again. Here we compare a SMOOTHED efficacy
 *     (EMA) against the baseline — one lucky tick neither triggers a harvest nor
 *     sets an unbeatable bar — and `lastHarvestEff` is set from the EMA too.
 */

/** EMA update — robust efficacy baseline a single-tick spike can't poison. */
export function emaUpdate(prev, sample, alpha = 0.2) {
  if (prev == null || !Number.isFinite(prev)) return sample
  return prev * (1 - alpha) + sample * alpha
}

/**
 * Decide whether to harvest the nursery body's learning into the master.
 * Returns { check, harvest, improved, forced }.
 *   - check   : throttle window elapsed; caller should reset its lastCheck clock.
 *   - harvest : perform the harvest (improved || forced).
 *   - improved: smoothed efficacy rose past the baseline by ≥ minGain.
 *   - forced  : it has been ≥ harvestMaxS since the last ACTUAL harvest.
 */
export function decideHarvest({
  now, lastCheck, lastHarvest, emaEff, lastHarvestEff,
  harvestS, harvestMaxS, minGain,
}) {
  if (now - lastCheck < harvestS * 1000) {
    return { check: false, harvest: false, improved: false, forced: false }
  }
  const improved = emaEff >= lastHarvestEff + minGain
  const forced   = now - lastHarvest >= harvestMaxS * 1000
  return { check: true, harvest: improved || forced, improved, forced }
}
