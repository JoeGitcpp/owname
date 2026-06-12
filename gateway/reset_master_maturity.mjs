#!/usr/bin/env node
/**
 * reset_master_maturity.mjs — one-time fix: clear the master's CONTAMINATED
 * developmental maturity.
 *
 * Background: the master learned_bias carries an illusory maturity (~0.9999)
 * from before the maturity-gating fix. Seeding branches from it makes every
 * branch a "fake-mature newborn" and the mainline's developmental clock starts
 * already pinned. This zeroes just the maturity so the lineage re-grows its
 * clock legitimately from the now-correct learning — biases / vocal_efficacy are
 * left intact.
 *
 * learned_bias.bin v5 tail = [..., prediction_familiarity, action_suppression,
 * vocal_efficacy, developmental_maturity] — maturity is the LAST f64.
 *
 * Usage:  node reset_master_maturity.mjs [path-to-master_learned_bias.bin]
 *   default path: ../master_learned_bias.bin (matches server.mjs MASTER_BIAS)
 *   set MATURITY_RESET_VALUE env to a non-zero start if desired (default 0).
 */
import { readFileSync, writeFileSync, copyFileSync, existsSync } from 'node:fs'
import { resolve } from 'node:path'

const path = process.argv[2] || resolve('../master_learned_bias.bin')
const resetTo = Number(process.env.MATURITY_RESET_VALUE || '0')

if (!existsSync(path)) {
  console.error(`[reset] master file not found: ${path}`)
  process.exit(1)
}
const buf = readFileSync(path)
if (buf.length < 8) {
  console.error(`[reset] file too small to hold maturity (${buf.length} bytes): ${path}`)
  process.exit(1)
}
const before = buf.readDoubleLE(buf.length - 8)

// Backup first — never destroy the only copy.
const backup = `${path}.bak-${Date.now()}`
copyFileSync(path, backup)

buf.writeDoubleLE(resetTo, buf.length - 8)
writeFileSync(path, buf)

console.log(`[reset] ${path}`)
console.log(`[reset] maturity ${before.toFixed(6)} -> ${resetTo.toFixed(6)}  (backup: ${backup})`)
console.log(`[reset] biases / vocal_efficacy left intact. Restart the mainline to re-grow the clock.`)
