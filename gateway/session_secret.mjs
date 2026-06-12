import { randomBytes } from 'node:crypto'
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'node:fs'
import { resolve, dirname } from 'node:path'

// Stable session-signing secret shared by email + phone auth. Priority:
//   1. ABORN_SESSION_SECRET env  (production should set this)
//   2. a persistent file         (so a restart does NOT log everyone out)
//   3. generate once, persist     (with a loud warning)
// Previously each auth module did `env || randomBytes(32)` independently, so an
// unset env meant a NEW random secret on every process start (all cookies
// invalidated on restart) AND email/phone using DIFFERENT secrets. This makes
// the secret stable and shared.
function resolveSecret() {
  if (process.env.ABORN_SESSION_SECRET) return process.env.ABORN_SESSION_SECRET
  const path = process.env.ABORN_SESSION_SECRET_FILE || resolve('../.session_secret')
  try {
    if (existsSync(path)) {
      const s = readFileSync(path, 'utf8').trim()
      if (s) return s
    }
    const s = randomBytes(32).toString('hex')
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, s, { mode: 0o600 })
    console.warn(`[auth] ABORN_SESSION_SECRET unset — generated and persisted a fallback at ${path}; set the env var in production.`)
    return s
  } catch {
    console.warn('[auth] ABORN_SESSION_SECRET unset and fallback file unwritable — sessions reset on restart.')
    return randomBytes(32).toString('hex')
  }
}

export const SESSION_SECRET = resolveSecret()
