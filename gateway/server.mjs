/**
 * aborn-gateway — session manager and sensory bridge.
 *
 * Each WebSocket connection = one body process with its own runtime dir.
 * Binary protocol (opaque — no field names, no JSON):
 *
 * Browser → gateway (sensory input, ~20 Hz):
 *   [0x01][audio_level:f32le][audio_flux:f32le][bands:12×f32le]
 *         [motion:f32le][luma:f32le][grid:768×f32le]
 *   Exact size: 3129 bytes
 *
 * Gateway → browser (vocal PCM, on demand):
 *   [0x02][n:u16le][samples:n×f32le]
 *
 * Gateway → browser (life-state pulse, ~1 Hz):
 *   [0x03][reserve:f32le][vital_state:u8][self_coherence:f32le][gap_pressure:f32le]
 *   Exact size: 10 bytes
 *
 * Security layers (see security.mjs for primitives):
 *   1. CF Access (Zero Trust) — upstream: unauthorized users blocked before
 *      reaching this process (set up in Cloudflare dashboard).
 *   2. Per-IP rate limiting — max N WebSocket upgrades per minute.
 *   3. Session guard — global cap + per-IP concurrent session cap.
 *   4. Frame validation — every browser frame checked for tag + exact size.
 *   5. Idle timeout — sessions with no valid frame for N minutes auto-close.
 *   6. Hard session limit — sessions older than MAX_SESSION_MIN are closed.
 *   7. No payload logging — event types only (no frame content, no IP logged).
 */

import { createServer }    from 'node:http'
import { WebSocketServer } from 'ws'
import { createConnection } from 'node:net'
import { spawn }            from 'node:child_process'
import {
  readFileSync, existsSync, mkdirSync, rmSync, copyFileSync,
  appendFileSync, readdirSync, renameSync,
}                           from 'node:fs'
import { join, resolve, dirname } from 'node:path'
import { fileURLToPath }    from 'node:url'
import { createReadStream } from 'node:fs'
import { extname }          from 'node:path'
import { createHash }       from 'node:crypto'
import { gzipSync }         from 'node:zlib'
import { existsSync as fileExists } from 'node:fs'

// ── Local env-file override (gateway.env in the deploy root) ─────────────────
// Lets operators set/override env vars without editing the systemd unit file.
// Format: KEY=VALUE, one per line; blank lines and # comments ignored.
// Lives at <WorkingDirectory>/../gateway.env  (i.e. /home/joe/aborn/gateway.env)
try {
  const envFile = new URL('../gateway.env', import.meta.url).pathname
  const { readFileSync: _rfs } = await import('node:fs')
  for (const raw of _rfs(envFile, 'utf8').split('\n')) {
    const line = raw.trim()
    if (!line || line.startsWith('#')) continue
    const eq = line.indexOf('=')
    if (eq < 1) continue
    const k = line.slice(0, eq).trim()
    const v = line.slice(eq + 1).trim()
    if (k) process.env[k] = v  // gateway.env overrides unit-file defaults
  }
} catch { /* gateway.env absent — fine */ }

// ── File logger (StandardOutput=null in systemd, so write to file) ────────────
// Logs connection events, auth, body lifecycle. Never logs frame content.
const LOG_FILE = join(new URL('.', import.meta.url).pathname, '../gateway.log')
function glog(...args) {
  const line = `[${new Date().toISOString()}] ${args.join(' ')}\n`
  process.stderr.write(line)                 // → journalctl (StandardError=journal)
  try { appendFileSync(LOG_FILE, line) } catch { } // → ~/aborn/gateway.log
}

import {
  validateFrame,
  RateLimiter,
  SessionGuard,
  IdleTracker,
  XorCipher,
  SENSES_FRAME_BYTES,
} from './security.mjs'
import {
  evaluate_session,
  collect_to_pool,
  merge_pool,
} from './training_pool.mjs'
import {
  handleAuthRequest,
  handleAuthVerify,
  handleAuthCheck,
  requireAuth,
  verifySessionCookie,
} from './auth.mjs'
import {
  handlePhoneRequest,
  handlePhoneVerify,
  verifyPhoneCookie,
} from './phone_auth.mjs'
import { InstanceRegistry }        from './instance_registry.mjs'
import { transition }               from './instance_lifecycle.mjs'
import { ResourceGovernor }        from './resource_governor.mjs'
import { MemoryStore }             from './memory_store.mjs'
import { EphemeralSessionManager } from './ephemeral_session.mjs'

// ── Config ────────────────────────────────────────────────────────────────────
const PORT             = parseInt(process.env.ABORN_PORT     || '3900',   10)
const BODY_BIN         = process.env.ABORN_BODY_BIN  || resolve('../bionic_v0/target/release/body')

// ── body sandbox (defence-in-depth) ──────────────────────────────────────────
// Visitor frames are attacker-controlled input. body is 0-unsafe Rust (very low
// RCE surface), but as a second wall we run each visitor body inside a
// bubblewrap user-namespace sandbox: no network (unix socket only), a private
// tmpfs /tmp, read-only system + body binary, and the ONLY writable path is the
// session dir. A compromised body therefore cannot reach the source tree, SSH
// keys, the cloudflared tunnel credentials, other sessions, or the network.
// Set ABORN_SANDBOX=0 to disable (e.g. if bwrap is unavailable on a host).
const SANDBOX = (process.env.ABORN_SANDBOX ?? '1') !== '0'
const BWRAP   = process.env.ABORN_BWRAP || '/usr/bin/bwrap'
function wrapBodyCmd(runtimeDir, socketPath) {
  if (!SANDBOX || !fileExists(BWRAP)) return [BODY_BIN, [runtimeDir, '--headless', socketPath]]
  // Inside the sandbox the session dir is /session and the body binary is
  // /aborn/body. The unix socket lives inside the bound session dir, so the
  // gateway (outside) still reaches it at the real socketPath.
  const sockName = socketPath.slice(runtimeDir.length).replace(/^\/+/, '') || 'net.sock'
  const args = [
    '--ro-bind', '/usr', '/usr',
    '--ro-bind', '/lib', '/lib',
    '--ro-bind', '/lib64', '/lib64',
    '--ro-bind', '/etc/ld.so.cache', '/etc/ld.so.cache',
    '--ro-bind', BODY_BIN, '/aborn/body',
    '--bind', runtimeDir, '/session',
    '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp',
    '--unshare-all',        // no net / ipc / pid / user namespaces shared
    '--die-with-parent',    // body dies if gateway dies
    '--new-session',        // no controlling tty (blocks TIOCSTI injection)
    '/aborn/body', '/session', '--headless', `/session/${sockName}`,
  ]
  return [BWRAP, args]
}
const SESSIONS_DIR     = process.env.ABORN_SESSIONS  || '/tmp/aborn-sessions'
const STATIC_DIR       = process.env.ABORN_STATIC    || resolve('../site/dist')
const GLOBAL_MAX       = parseInt(process.env.ABORN_MAX      || '8',      10)
const PER_IP_MAX       = parseInt(process.env.ABORN_PERIP    || '2',      10)
const RATE_PER_MIN     = parseInt(process.env.ABORN_RATE     || '5',      10)
const IDLE_MIN         = parseInt(process.env.ABORN_IDLE_MIN || '5',      10)
const MAX_SESSION_MIN  = parseInt(process.env.ABORN_MAX_MIN  || '30',     10)
// Training pool: where collected learned_bias files go, and where the master lives
const POOL_DIR         = process.env.ABORN_POOL_DIR  || resolve('../training_pool')
const MASTER_BIAS      = process.env.ABORN_MASTER_BIAS || resolve('../master_learned_bias.bin')
const MASTER_RUNTIME   = process.env.ABORN_RUNTIME     || resolve('../runtime')
// Per-user persistent memory: familiar_objects, vital_core, self_model survive across sessions.
// Keyed by SHA-256(email)[0:16] — no raw email on disk.
const USERS_DIR        = process.env.ABORN_USERS_DIR  || resolve('../user-memories')
// Merge the pool into master every N minutes (0 = disable)
const MERGE_INTERVAL_MIN = parseInt(process.env.ABORN_MERGE_MIN || '60', 10)
// Persistent instances: max simultaneously active instances (hot-updatable via POST /admin/instances/limit)
const INSTANCE_MAX          = parseInt(process.env.ABORN_INSTANCE_MAX   || '8',  10)
// Ephemeral sessions: per-IP concurrent cap and idle timeout
const EPHEMERAL_PERIP_MAX   = parseInt(process.env.ABORN_EPH_PERIP      || '3',  10)
const EPHEMERAL_TIMEOUT_MIN = parseInt(process.env.ABORN_EPH_IDLE_MIN   || '30', 10)

// Personal checkpoint files that belong to the user and must never be merged into master.
const PERSONAL_FILES = ['familiar_objects.bin', 'vital_core.bin', 'self_model.bin']

// Mainline GENERAL skill (vocal repertoire + cross-layer abstraction) seeded into
// every branch so a new conversation starts at the CURRENT mainline level rather
// than from zero. Stored as master_<name>.bin alongside MASTER_BIAS, written by
// the mainline's harvest (nursery_feeder MASTER_ARTIFACTS). These are shared
// skill — distinct from PERSONAL_FILES (relationship, kept per-user).
const MASTER_DIR = dirname(MASTER_BIAS)
const MAINLINE_VOCAL_FILES = [
  'phoneme_memory.bin',
  'vocal_memory.bin',
  'vocal_target_recall.bin',
  'snapshot.bin',
]
// Prototype vocal artifacts harvested branch→pool→mainline by structure-aware
// MERGE (merge_artifacts). snapshot is seed-only (not merged back). learned_bias
// is harvested separately via collect_to_pool + EMA merge_pool.
const MAINLINE_HARVEST_FILES = ['vocal_target_recall.bin', 'vocal_memory.bin', 'phoneme_memory.bin']
// Rust tool that merges pooled branch artifacts into master_<name>.bin.
const MERGE_ARTIFACTS_BIN = process.env.ABORN_MERGE_BIN
  || resolve('../bionic_v0/target/release/merge_artifacts')

mkdirSync(SESSIONS_DIR, { recursive: true })
mkdirSync(USERS_DIR,    { recursive: true })

// Sweep stale session dirs left from a previous crash.
// Safe at startup: activeSessions is still empty, no live bodies yet.
try {
  let swept = 0
  for (const entry of readdirSync(SESSIONS_DIR, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      rmSync(join(SESSIONS_DIR, entry.name), { recursive: true, force: true })
      swept++
    }
  }
  if (swept > 0) glog(`[startup] swept ${swept} stale session dir(s)`)
} catch { /* SESSIONS_DIR empty or unreadable — ignore */ }

// ── Security instances ────────────────────────────────────────────────────────
const rateLimiter = new RateLimiter({ maxPerMinute: RATE_PER_MIN, windowMs: 60_000 })
const sessionGuard = new SessionGuard({ globalMax: GLOBAL_MAX, perIpMax: PER_IP_MAX })
const idleTracker  = new IdleTracker({ idleMs: IDLE_MIN * 60_000 })

// ── Instance management singletons ───────────────────────────────────────────
const instanceRegistry = new InstanceRegistry()
const resourceGovernor = new ResourceGovernor(INSTANCE_MAX)
const memoryStore      = new MemoryStore(USERS_DIR)
const ephemeralMgr     = new EphemeralSessionManager({
  perIpMax:  EPHEMERAL_PERIP_MAX,
  timeoutMs: EPHEMERAL_TIMEOUT_MIN * 60_000,
})

// ── Shared HTTP helpers ───────────────────────────────────────────────────────
function readBody(req) {
  return new Promise(resolve => {
    let body = ''
    req.on('data', c => body += c)
    req.on('end', () => resolve(body))
  })
}

function sendJson(res, status, data) {
  res.writeHead(status, { 'Content-Type': 'application/json' })
  res.end(JSON.stringify(data))
}

/** Extract the real client IP, honouring Cloudflare and standard proxy headers. */
function clientIp(req) {
  return req.headers['cf-connecting-ip']
      || req.headers['x-forwarded-for']?.split(',')[0]?.trim()
      || req.socket.remoteAddress
      || 'unknown'
}

/**
 * Resolve the caller's identity from their session cookies.
 * Checks phone cookie first (new), then email cookie (legacy).
 * Returns { key, type, value } or null.
 */
function resolveIdentity(req) {
  const phone = verifyPhoneCookie(req.headers.cookie)
  if (phone?.phone) return { key: `phone:${phone.phone}`, type: 'phone', value: phone.phone }
  const email = verifySessionCookie(req.headers.cookie)
  if (email?.email) return { key: `email:${email.email}`, type: 'email', value: email.email }
  return null
}

// Periodic cleanup: prune rate-limiter + close idle sessions
setInterval(() => {
  rateLimiter.prune()
  for (const id of idleTracker.idleSessions()) {
    const entry = activeSessions.get(id)
    if (entry) entry.ws.close(1001, 'idle')
  }
}, 60_000)

// Periodic training-pool merge (if enabled)
if (MERGE_INTERVAL_MIN > 0) {
  setInterval(() => {
    try {
      const r = merge_pool(POOL_DIR, MASTER_BIAS)
      if (r.merged > 0) {
        console.log(`[pool] merged ${r.merged} sessions into master (skipped ${r.skipped})`)
      }
    } catch { /* pool not ready yet */ }
    // Increment 3: fold pooled branch VOCAL artifacts into the mainline by
    // structure-aware merge (single-threaded here → no race with branches that
    // only write into the pool). Spawned tool consumes the pool files it merges.
    try {
      const m = spawn(MERGE_ARTIFACTS_BIN, [MASTER_DIR, POOL_DIR], { stdio: 'ignore' })
      m.on('error', () => { /* tool missing / not built — skip */ })
    } catch { /* skip */ }
  }, MERGE_INTERVAL_MIN * 60_000)
}

// ── MIME types ────────────────────────────────────────────────────────────────
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js':   'application/javascript',
  '.css':  'text/css',
  '.svg':  'image/svg+xml',
  '.png':  'image/png',
  '.ico':  'image/x-icon',
  '.woff2':'font/woff2',
  '.json': 'application/json',
}

// ── Active sessions (id → { ws, process, socketPath, runtimeDir, startedAt }) ─
const activeSessions = new Map()

// ── HTTP server ───────────────────────────────────────────────────────────────
const http = createServer(async (req, res) => {
  const url = req.url.split('?')[0]

  // ── Auth routes ─────────────────────────────────────────────────────────────
  if (url === '/auth/request' && req.method === 'POST') return handleAuthRequest(req, res)
  if (url === '/auth/verify'  && req.method === 'POST') return handleAuthVerify(req, res)
  if (url === '/auth/check')                            return handleAuthCheck(req, res)
  if (url === '/auth/request-phone' && req.method === 'POST') return handlePhoneRequest(req, res)
  if (url === '/auth/verify-phone'  && req.method === 'POST') return handlePhoneVerify(req, res)

  // ── Instance routes (/instances) ─────────────────────────────────────────
  // GET /instances/me — return caller's current instance (null if none)
  if (url === '/instances/me' && req.method === 'GET') {
    const identity = resolveIdentity(req)
    if (!identity) return sendJson(res, 401, { error: 'unauthorized' })
    const instance = instanceRegistry.getByIdentity(identity.key)
    return sendJson(res, 200, { instance: instance ?? null })
  }

  // POST /instances — create a persistent instance for the authenticated user
  if (url === '/instances' && req.method === 'POST') {
    const identity = resolveIdentity(req)
    if (!identity) return sendJson(res, 401, { error: 'unauthorized' })

    if (!resourceGovernor.canStart()) {
      return sendJson(res, 503, {
        error:   'capacity_full',
        message: '当前实例容量已满，请稍后重试',
        queued:  resourceGovernor.suspended(),
      })
    }
    try {
      const inst     = instanceRegistry.create(identity.key)
      const newState = transition(inst.state, 'verify')   // pending_verification → running
      instanceRegistry.setState(inst.id, newState)
      resourceGovernor.onStart(inst.id)
      glog(`[instance] created: id=${inst.id} identity=${identity.key}`)
      return sendJson(res, 201, { instance: instanceRegistry.get(inst.id) })
    } catch (e) {
      if (/already/i.test(e.message)) return sendJson(res, 409, { error: 'instance_exists' })
      throw e
    }
  }

  // DELETE /instances/me — destroy the caller's instance and free its slot
  if (url === '/instances/me' && req.method === 'DELETE') {
    const identity = resolveIdentity(req)
    if (!identity) return sendJson(res, 401, { error: 'unauthorized' })
    const inst = instanceRegistry.getByIdentity(identity.key)
    if (!inst) return sendJson(res, 404, { error: 'no_instance' })

    const nextToResume = resourceGovernor.onDestroy(inst.id)
    instanceRegistry.destroy(inst.id)
    if (nextToResume) {
      // Resume oldest queued instance — bring it back to running
      try { instanceRegistry.setState(nextToResume, 'running') } catch { }
      glog(`[instance] resumed from queue: id=${nextToResume}`)
    }
    glog(`[instance] destroyed: id=${inst.id} identity=${identity.key}`)
    return sendJson(res, 200, { ok: true })
  }

  // ── Admin: hot-update the instance cap ──────────────────────────────────
  // POST /admin/instances/limit  { limit: number }
  // Protected by ABORN_ADMIN_SECRET header.
  if (url === '/admin/instances/limit' && req.method === 'POST') {
    const secret = process.env.ABORN_ADMIN_SECRET
    if (!secret || req.headers['x-admin-secret'] !== secret) {
      return sendJson(res, 403, { error: 'forbidden' })
    }
    const body = await readBody(req)
    let limit
    try { limit = parseInt(JSON.parse(body).limit, 10) } catch { }
    if (!Number.isInteger(limit) || limit < 0) {
      return sendJson(res, 400, { error: 'invalid_limit' })
    }
    const toSuspend = resourceGovernor.setLimit(limit)
    for (const id of toSuspend) {
      try { instanceRegistry.setState(id, 'suspended') } catch { }
    }
    glog(`[admin] instance limit updated: ${limit} (suspended ${toSuspend.length})`)
    return sendJson(res, 200, {
      ok:        true,
      limit:     resourceGovernor.limit,
      running:   resourceGovernor.running(),
      suspended: resourceGovernor.suspended(),
    })
  }

  // ── Session routes (/sessions) — ephemeral, no auth required ─────────────
  // POST /sessions — create an ephemeral session slot
  if (url === '/sessions' && req.method === 'POST') {
    const ip = clientIp(req)
    try {
      const sessionId = ephemeralMgr.create(ip)
      return sendJson(res, 201, { sessionId })
    } catch {
      return sendJson(res, 429, {
        error:   'session_limit',
        message: '当前临时会话数已达上限',
      })
    }
  }

  // DELETE /sessions/:id — explicitly destroy an ephemeral session
  if (url.startsWith('/sessions/') && req.method === 'DELETE') {
    const sessionId = url.slice('/sessions/'.length)
    if (sessionId) ephemeralMgr.destroy(sessionId)
    return sendJson(res, 200, { ok: true })
  }

  // GET /sessions/stats — diagnostic (active count, per-IP)
  if (url === '/sessions/stats' && req.method === 'GET') {
    const ip = clientIp(req)
    return sendJson(res, 200, {
      total:     ephemeralMgr.size(),
      forThisIp: ephemeralMgr.countForIp(ip),
    })
  }

  let path = url
  if (path === '/' || !path.includes('.')) path = '/index.html'

  const file = join(STATIC_DIR, path)
  const ext  = extname(file)
  const mime = MIME[ext] || 'application/octet-stream'

  // Security: prevent path traversal
  if (!file.startsWith(STATIC_DIR)) {
    res.writeHead(403); res.end(); return
  }

  if (!existsSync(file)) {
    const idx = join(STATIC_DIR, 'index.html')
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' })
    createReadStream(idx).pipe(res)
    return
  }

  res.writeHead(200, {
    'Content-Type': mime,
    // Security headers
    'X-Content-Type-Options': 'nosniff',
    'X-Frame-Options': 'DENY',
    'Referrer-Policy': 'no-referrer',
    'Permissions-Policy': 'camera=(self), microphone=(self)',
  })
  createReadStream(file).pipe(res)
})

// ── WebSocket server ──────────────────────────────────────────────────────────
const wss = new WebSocketServer({
  server: http,
  maxPayload: SENSES_FRAME_BYTES + 64,
  // Cloudflare tunnel strips WebSocket extension headers (permessage-deflate),
  // causing the binary XOR hello to arrive garbled at the client.
  // Disable compression so all frames are sent as raw binary.
  perMessageDeflate: false,
})

// ── Server-side ping keepalive (mobile lock-screen guard) ─────────────────────
// Mobile browsers (iOS Safari, Android Chrome) silently drop the WebSocket TCP
// connection when the screen locks or the app goes to background.  The OS does
// not send a proper WS close frame, so the server sees a sudden 1006 drop after
// the NAT/firewall idle timeout (typically 30-90 s).
//
// Send a ping frame every 20 s to reset NAT idle timers on the CF tunnel →
// server TCP leg.  Cloudflare strips WS control frames before they reach the
// browser, so pongs never come back — do NOT terminate on missed pong or every
// session dies at 20–40 s.
const PING_INTERVAL_MS = 20_000
setInterval(() => {
  for (const client of wss.clients) {
    if (client.readyState !== client.OPEN) continue
    client.ping()
  }
}, PING_INTERVAL_MS)

wss.on('connection', (ws, req) => {

  // ── Identity resolution: phone cookie > email cookie > anon ──────────────
  const identity    = resolveIdentity(req)
  const authSession = verifySessionCookie(req.headers.cookie)  // keep for legacy check

  if (!identity && !authSession && requireAuth(req)) {
    glog('[ws] rejected: no valid session — auth required')
    ws.close(1008, 'unauthorized')
    return
  }
  glog(`[ws] connected: user=${identity?.value ?? authSession?.email ?? 'anon'}`)

  // ── User directory for binary personal files ──────────────────────────────
  // New: keyed by SHA-256(identityValue)[0:16] for both email and phone.
  // Legacy email sessions continue to use SHA-256(email) — same derivation.
  const identityValue = identity?.value ?? authSession?.email ?? null
  const userKey = identityValue
    ? createHash('sha256').update(identityValue.toLowerCase()).digest('hex').slice(0, 16)
    : 'anon'
  const userDir = join(USERS_DIR, userKey)
  if (identityValue) mkdirSync(userDir, { recursive: true })

  // ── Ephemeral session registration (anon only) ───────────────────────────
  // Persistent-instance connections are governed by instanceRegistry + resourceGovernor.
  // Anonymous connections use EphemeralSessionManager to enforce per-IP cap and idle sweep.
  let ephemeralId = null
  const ip = clientIp(req)

  if (!identity) {
    try {
      ephemeralId = ephemeralMgr.create(ip)
    } catch {
      ws.close(1013, 'ephemeral-capacity')
      return
    }
  }

  // Layer 2: rate limit WS upgrades per IP
  if (!rateLimiter.allow(ip)) {
    ws.close(1008, 'rate-limit')
    return
  }

  // Layer 3: session cap (global + per-IP)
  const sessionId = sessionGuard.admit(ip)
  if (!sessionId) {
    ws.close(1013, 'capacity')
    return
  }

  const runtimeDir = join(SESSIONS_DIR, sessionId)
  const socketPath = join(runtimeDir, 'net.sock')
  mkdirSync(runtimeDir, { recursive: true })

  // Seed new session:
  //   learned_bias.bin  ← master (shared general experience, best available)
  //   personal files    ← user's own dir first, fallback to master runtime
  //   This way the body starts with its general skills AND remembers this user.
  const SEED_FILES = [
    ['learned_bias.bin', existsSync(MASTER_BIAS)
      ? MASTER_BIAS
      : join(MASTER_RUNTIME, 'learned_bias.bin')],
  ]
  for (const f of PERSONAL_FILES) {
    const fromUser   = join(userDir, f)
    const fromMaster = join(MASTER_RUNTIME, f)
    SEED_FILES.push([f, existsSync(fromUser) ? fromUser : fromMaster])
  }
  // Mainline general skill: seed each branch from master_<name>.bin so it starts
  // at the current mainline vocal/abstraction level (not from zero). Falls back
  // to MASTER_RUNTIME/<name> if the master_ file isn't present yet.
  for (const f of MAINLINE_VOCAL_FILES) {
    const fromMaster = join(MASTER_DIR, 'master_' + f)
    const fallback   = join(MASTER_RUNTIME, f)
    SEED_FILES.push([f, existsSync(fromMaster) ? fromMaster : fallback])
  }
  for (const [dst, src] of SEED_FILES) {
    if (existsSync(src)) {
      try { copyFileSync(src, join(runtimeDir, dst)) } catch { }
    }
  }

  // Spawn body in headless mode
  const modeLabel = identity ? `persistent:${identity.key}` : 'ephemeral'
  glog(`[body] spawn: session=${sessionId} mode=${modeLabel}`)
  const [spawnBin, spawnArgs] = wrapBodyCmd(runtimeDir, socketPath)
  const child = spawn(spawnBin, spawnArgs, {
    stdio: 'ignore',
    detached: false,
  })

  const entry = {
    ws,
    process:    child,
    socketPath,
    runtimeDir,
    userDir,
    startedAt:  Date.now(),
  }
  activeSessions.set(sessionId, entry)
  idleTracker.add(sessionId)

  // Hard session timeout
  const hardTimeout = setTimeout(() => {
    ws.close(1001, 'session-limit')
  }, MAX_SESSION_MIN * 60_000)

  // XOR session key — send hello to browser, apply to all frames both ways
  const xorKey = XorCipher.newKey()
  const hello  = Buffer.allocUnsafe(5)
  hello[0] = 0x00   // hello tag
  xorKey.copy(hello, 1)
  ws.send(hello)    // browser reads this and initialises its XOR state

  // Connect to body's Unix socket
  let bodySocket   = null
  let vocalBuf     = Buffer.alloc(0)
  let stateTimer   = null
  let badFrames    = 0
  const MAX_BAD    = 10  // disconnect after N consecutive bad frames

  // Backpressure-aware surface relay. The body emits raw 320×180 RGBA surface
  // frames (~230 KB) at ~30 fps (~7 MB/s). Forwarding every frame queues
  // unboundedly in the ws send buffer when the tunnel/client link is slower,
  // delaying ALL frames (vocal audio included) by seconds-to-minutes — the
  // browser sees a black surface and hears nothing. Instead: keep only the
  // freshest surface frame and flush it when the socket has drained; drop
  // stale ones. Vocal frames are small and latency-sensitive — forward
  // immediately, dropping only under heavy backlog.
  let latestSurface  = null
  let surfaceTimer   = null
  const SURFACE_FLUSH_MS      = 250         // ≤ 4 fps to the browser
  const SURFACE_BACKLOG_MAX   = 64 * 1024   // flush only when queue is short
  const VOCAL_BACKLOG_MAX     = 256 * 1024  // drop vocal only past this
  // Tunnel goodput measured at ~100 KB/s: a raw 320×180 RGBA frame (230 KB)
  // alone exceeds it. Downsample 4× (80×45, ~14 KB) before relaying — the
  // client draws whatever w×h the frame declares and CSS scales it up.
  const SURFACE_DOWNSCALE     = 4
  const downsampleSurface = (frame) => {
    const w = frame.readUInt16LE(1), h = frame.readUInt16LE(3)
    const f = SURFACE_DOWNSCALE
    if (f <= 1 || w % f || h % f) return frame
    const dw = w / f, dh = h / f
    const out = Buffer.allocUnsafe(5 + dw * dh * 4)
    out[0] = frame[0]
    out.writeUInt16LE(dw, 1)
    out.writeUInt16LE(dh, 3)
    for (let y = 0; y < dh; y++) {
      for (let x = 0; x < dw; x++) {
        const src = 5 + ((y * f) * w + (x * f)) * 4
        const dst = 5 + (y * dw + x) * 4
        frame.copy(out, dst, src, src + 4)
      }
    }
    return out
  }

  const tryConnect = (attempts = 0) => {
    if (attempts > 40 || ws.readyState !== 1 /* OPEN */) {
      ws.close(1011, 'body-timeout'); return
    }
    if (!existsSync(socketPath)) {
      setTimeout(() => tryConnect(attempts + 1), 250); return
    }
    const c = createConnection(socketPath)
    c.on('error', () => setTimeout(() => tryConnect(attempts + 1), 250))
    c.once('connect', () => {
      bodySocket = c

      c.on('data', (chunk) => {
        vocalBuf = Buffer.concat([vocalBuf, chunk])
        while (vocalBuf.length >= 3) {
          const tag = vocalBuf[0]
          if (tag === 0x02) {
            const n    = vocalBuf.readUInt16LE(1)
            const need = 3 + n * 4
            if (vocalBuf.length < need) break
            if (ws.readyState === 1 && ws.bufferedAmount < VOCAL_BACKLOG_MAX) {
              // Convert f32 → int16 before sending: halves bandwidth (~51 KB/s vs ~103 KB/s).
              // Format stays [0x02][n:u16le][n×i16le] — browser reads getInt16/32768.
              const frame = Buffer.allocUnsafe(3 + n * 2)
              frame[0] = 0x02
              frame.writeUInt16LE(n, 1)
              for (let i = 0; i < n; i++) {
                const s = vocalBuf.readFloatLE(3 + i * 4)
                frame.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(s * 32767))), 3 + i * 2)
              }
              ws.send(XorCipher.apply(frame, xorKey))
            }
            vocalBuf = vocalBuf.slice(need)
          } else if (tag === 0x04) {
            // Surface pixel frame: [0x04][w:u16le][h:u16le][w*h*4 RGBA bytes]
            if (vocalBuf.length < 5) break
            const w    = vocalBuf.readUInt16LE(1)
            const h    = vocalBuf.readUInt16LE(3)
            const need = 5 + w * h * 4
            if (vocalBuf.length < need) break
            latestSurface = downsampleSurface(vocalBuf.slice(0, need))  // keep freshest, flushed by surfaceTimer
            vocalBuf = vocalBuf.slice(need)
          } else {
            vocalBuf = vocalBuf.slice(1)  // resync
          }
        }
      })

      c.on('close', () => { if (ws.readyState === 1) ws.close(1000) })
    })
  }
  setTimeout(() => tryConnect(), 300)

  // Flush the freshest surface frame whenever the socket has drained. The frame
  // is gzip-compressed before sending (tag 0x05): downsampled RGBA pixels are
  // highly compressible (large flat/black regions), typically 14 KB → ~2-4 KB,
  // a 4-6× bandwidth cut over the ~100 KB/s tunnel. The client gunzips back to
  // the raw 0x04 frame. Clients that predate 0x05 still get nothing worse than
  // before; the gateway only emits 0x05 once both ends are deployed.
  surfaceTimer = setInterval(() => {
    if (!latestSurface || ws.readyState !== 1) return
    if (ws.bufferedAmount > SURFACE_BACKLOG_MAX) return  // still draining — retry next tick
    const gz = gzipSync(latestSurface, { level: 6 })
    const framed = Buffer.allocUnsafe(1 + gz.length)
    framed[0] = 0x05            // gzip-compressed surface frame
    gz.copy(framed, 1)
    ws.send(XorCipher.apply(framed, xorKey))
    latestSurface = null
  }, SURFACE_FLUSH_MS)

  // Life-state pulse at ~1 Hz
  stateTimer = setInterval(() => {
    if (ws.readyState !== 1) return
    const vcPath = join(runtimeDir, 'vital_core.bin')
    if (!existsSync(vcPath)) return
    try {
      const vc = readFileSync(vcPath)
      if (vc.length < 41) return
      const view    = new DataView(vc.buffer, vc.byteOffset, vc.byteLength)
      const reserve = view.getFloat64(16, true)
      const vstate  = vc[40]
      const frame   = Buffer.allocUnsafe(10)
      frame[0]      = 0x03
      frame.writeFloatLE(Math.min(1, Math.max(0, reserve || 0)), 1)
      frame[5]      = vstate || 0
      frame.writeFloatLE(0, 6)  // self_coherence (future)
      frame.writeFloatLE(0, 10 - 4)  // gap_pressure (future)
      ws.send(XorCipher.apply(frame, xorKey))
    } catch { /* snapshot not ready */ }
  }, 1000)

  // Layer 4 + 5: validate every incoming frame; track idle
  ws.on('message', (data, isBinary) => {
    if (!isBinary) {
      glog(`[ws] close: session=${sessionId} reason=text-not-allowed`)
      ws.close(1003, 'text-not-allowed'); return
    }

    const raw = Buffer.isBuffer(data) ? data : Buffer.from(data)
    // Decode XOR obfuscation before validation and forwarding
    const buf = XorCipher.apply(raw, xorKey)

    if (!validateFrame(buf)) {
      badFrames++
      if (badFrames === 1) glog(`[ws] bad-frame: session=${sessionId} size=${raw.length} tag=0x${buf[0]?.toString(16)}`)
      if (badFrames >= MAX_BAD) {
        glog(`[ws] close: session=${sessionId} reason=bad-frames count=${badFrames}`)
        ws.close(1008, 'bad-frames')
      }
      return
    }
    badFrames = 0
    idleTracker.touch(sessionId)
    if (ephemeralId) ephemeralMgr.heartbeat(ephemeralId)   // reset idle timer for ephemeral

    try { bodySocket?.write(buf) } catch { /* body socket closed */ }
  })

  ws.on('close', (code, reason) => {
    glog(`[ws] closed: session=${sessionId} code=${code} reason=${reason?.toString()||''}`)
  })

  const cleanup = () => {
    clearTimeout(hardTimeout)
    clearInterval(stateTimer)
    clearInterval(surfaceTimer)
    idleTracker.remove(sessionId)
    sessionGuard.release(sessionId)
    activeSessions.delete(sessionId)
    if (ephemeralId) ephemeralMgr.destroy(ephemeralId)  // free per-IP slot immediately

    // Give body 3 s to write checkpoint, then evaluate and collect.
    setTimeout(() => {
      child.kill('SIGTERM')
      setTimeout(() => {
        // Save personal memory files back to user's persistent dir BEFORE any cleanup.
        // Persistent-instance sessions save back to their named userDir.
        // Ephemeral sessions write to 'anon' dir which is the shared fallback.
        if (existsSync(userDir)) {
          for (const f of PERSONAL_FILES) {
            const src = join(runtimeDir, f)
            if (existsSync(src)) {
              try { copyFileSync(src, join(userDir, f)) } catch { }
            }
          }
        }

        // Evaluate session quality before collecting training result.
        // Only learned_bias.bin goes to the pool — general experience, no personal data.
        try {
          const result = evaluate_session({
            runtimeDir,
            startedAt: entry.startedAt,
            endedAt: Date.now(),
          })
          if (result.pass) {
            collect_to_pool(runtimeDir, POOL_DIR, sessionId)
            // Increment 3: harvest the branch's VOCAL learning into the pool too,
            // to be merged (not overwritten) into the mainline by merge_artifacts.
            // Named {ts}_{sid}__<name>.bin to match that tool's glob. learned_bias
            // is handled by collect_to_pool above; snapshot is seed-only.
            const stamp = `${Date.now()}_${sessionId}`
            for (const f of MAINLINE_HARVEST_FILES) {
              const src = join(runtimeDir, f)
              if (existsSync(src)) {
                // Write atomically: copy to .tmp then rename, so the periodic
                // merge_artifacts (which globs *__<name>.bin) never reads a
                // half-written file and silently drops the harvest.
                const dst = join(POOL_DIR, `${stamp}__${f}`)
                try { copyFileSync(src, dst + '.tmp'); renameSync(dst + '.tmp', dst) } catch { }
              }
            }
          }
          // else: session did not meet quality threshold — discard
        } catch { /* evaluation error: skip collection */ }

        // Clean up session directory (personal files already saved above)
        setTimeout(() => {
          try { rmSync(runtimeDir, { recursive: true, force: true }) } catch { }
        }, 1000)
      }, 3000)
    }, 3000)
  }

  ws.on('close', cleanup)
  ws.on('error', () => ws.close())
  child.on('exit', (code) => {
    glog(`[body] exit: session=${sessionId} code=${code}`)
    activeSessions.delete(sessionId)
    if (ws.readyState === 1) ws.close(1000)
  })
})

http.listen(PORT, '127.0.0.1', () => {
  glog(`[aborn-gateway] :${PORT} bound to 127.0.0.1 (tunnel only)`)
  glog(`[aborn-gateway] body=${BODY_BIN} sessions=${SESSIONS_DIR}`)
  glog(`[aborn-gateway] harvest: relaxed=${process.env.ABORN_HARVEST_RELAXED === '1'} merge_min=${MERGE_INTERVAL_MIN}`)
})

// ── Continuous mainline life ────────────────────────────────────────────────
// A persistent server-side body that keeps growing and harvests into the master
// every HARVEST seconds; visitor branches seed from that master. Off by default
// (dev/test); enable in production with ABORN_MAINLINE=1. Supervised: respawn on
// exit with exponential backoff so a crash doesn't stop the mainline growing.
if (process.env.ABORN_MAINLINE === '1') {
  const FEEDER = fileURLToPath(new URL('./nursery_feeder.mjs', import.meta.url))
  const MAINLINE_RUNTIME = process.env.ABORN_MAINLINE_RUNTIME || resolve('../runtime-mainline')
  const SANDBOX_BIN = process.env.ABORN_SANDBOX_BIN || resolve('../bionic_v0/target/release/sandbox_runner')
  const HARVEST_S = process.env.ABORN_MAINLINE_HARVEST || '120'
  let backoffMs = 2000
  let healthyTimer = null
  const spawnMainline = () => {
    glog(`[mainline] spawning continuous life (master=${MASTER_BIAS}, runtime=${MAINLINE_RUNTIME})`)
    const args = [
      FEEDER, '--forever', '--master', MASTER_BIAS,
      '--body', BODY_BIN, '--sandbox', SANDBOX_BIN,
      '--runtime', MAINLINE_RUNTIME, '--harvest', String(HARVEST_S),
    ]
    const m = spawn(process.execPath, args, { stdio: 'ignore' })
    m.on('error', (e) => glog(`[mainline] spawn error: ${e.message}`))
    m.on('exit', (code) => {
      if (healthyTimer) clearTimeout(healthyTimer)
      glog(`[mainline] exited code=${code}; respawning in ${backoffMs}ms`)
      setTimeout(spawnMainline, backoffMs)
      backoffMs = Math.min(backoffMs * 2, 60_000) // cap 60s
    })
    // Reset backoff after it has stayed up a while (healthy run).
    healthyTimer = setTimeout(() => { backoffMs = 2000 }, 60_000)
  }
  spawnMainline()
}
