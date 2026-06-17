/**
 * session_archive.mjs — retain a session's diagnostic telemetry before its
 * runtime dir is deleted on cleanup, so a call can be reviewed after the fact
 * ("was it positive?"). Keeps only lightweight, NON-personal diagnostics
 * (vocal_diag.csv) plus a small meta.json — never the learning binaries.
 */
import {
  existsSync, mkdirSync, copyFileSync, writeFileSync, readdirSync, statSync, rmSync,
} from 'node:fs'
import { join } from 'node:path'

// Diagnostics worth keeping. Deliberately excludes snapshot.bin / learned_bias.bin
// / familiar_objects.bin etc. — those are large and/or personal.
//
// perturbation_graph.bin is small (~1–20 KB even after a long session) AND its
// node signatures are tier-0 statistics (HSV / gradient histogram / motion)
// rather than recoverable images — so it's safe to retain for after-the-fact
// inspection of "did anything visual form?" on a per-call basis. Anonymous
// sessions don't go through PERSONAL_FILES, so this is the only chance to
// inspect their visual cognition.
const DIAG_FILES = ['vocal_diag.csv', 'perturbation_graph.bin']

/**
 * Copy retained diagnostics from runtimeDir → archiveDir/<sessionId>/, plus a
 * meta.json. Best-effort: never throws (cleanup must proceed regardless).
 * Returns the list of archived file basenames (may be empty).
 */
export function archiveSessionDiag(runtimeDir, archiveDir, sessionId, meta = {}) {
  const archived = []
  try {
    const dest = join(archiveDir, String(sessionId))
    mkdirSync(dest, { recursive: true })
    for (const f of DIAG_FILES) {
      const src = join(runtimeDir, f)
      if (existsSync(src)) {
        try { copyFileSync(src, join(dest, f)); archived.push(f) } catch {}
      }
    }
    try {
      writeFileSync(
        join(dest, 'meta.json'),
        JSON.stringify({ sessionId: String(sessionId), archived, ...meta }, null, 2),
      )
    } catch {}
  } catch {}
  return archived
}

/**
 * Prune archived sessions older than maxAgeDays (by directory mtime).
 * Best-effort; returns count pruned.
 */
export function pruneSessionArchive(archiveDir, { maxAgeDays = 14, now = Date.now() } = {}) {
  let pruned = 0
  try {
    if (!existsSync(archiveDir)) return 0
    const cutoff = now - maxAgeDays * 86_400_000
    for (const name of readdirSync(archiveDir)) {
      const p = join(archiveDir, name)
      try {
        if (statSync(p).mtimeMs < cutoff) { rmSync(p, { recursive: true, force: true }); pruned++ }
      } catch {}
    }
  } catch {}
  return pruned
}
