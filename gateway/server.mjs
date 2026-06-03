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
  readFileSync, existsSync, mkdirSync, rmSync,
}                           from 'node:fs'
import { join, resolve }    from 'node:path'
import { createReadStream } from 'node:fs'
import { extname }          from 'node:path'

import {
  validateFrame,
  RateLimiter,
  SessionGuard,
  IdleTracker,
  SENSES_FRAME_BYTES,
} from './security.mjs'

// ── Config ────────────────────────────────────────────────────────────────────
const PORT             = parseInt(process.env.ABORN_PORT     || '3900',   10)
const BODY_BIN         = process.env.ABORN_BODY_BIN  || resolve('../bionic_v0/target/release/body')
const SESSIONS_DIR     = process.env.ABORN_SESSIONS  || '/tmp/aborn-sessions'
const STATIC_DIR       = process.env.ABORN_STATIC    || resolve('../site/dist')
const GLOBAL_MAX       = parseInt(process.env.ABORN_MAX      || '8',      10)
const PER_IP_MAX       = parseInt(process.env.ABORN_PERIP    || '2',      10)
const RATE_PER_MIN     = parseInt(process.env.ABORN_RATE     || '5',      10)
const IDLE_MIN         = parseInt(process.env.ABORN_IDLE_MIN || '5',      10)
const MAX_SESSION_MIN  = parseInt(process.env.ABORN_MAX_MIN  || '30',     10)

mkdirSync(SESSIONS_DIR, { recursive: true })

// ── Security instances ────────────────────────────────────────────────────────
const rateLimiter = new RateLimiter({ maxPerMinute: RATE_PER_MIN, windowMs: 60_000 })
const sessionGuard = new SessionGuard({ globalMax: GLOBAL_MAX, perIpMax: PER_IP_MAX })
const idleTracker  = new IdleTracker({ idleMs: IDLE_MIN * 60_000 })

// Periodic cleanup
setInterval(() => {
  rateLimiter.prune()
  // Force-close idle sessions
  for (const id of idleTracker.idleSessions()) {
    const entry = activeSessions.get(id)
    if (entry) entry.ws.close(1001, 'idle')
  }
}, 60_000)

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
const http = createServer((req, res) => {
  let path = req.url.split('?')[0]
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
    'Permissions-Policy': 'camera=(), microphone=(self)',
  })
  createReadStream(file).pipe(res)
})

// ── WebSocket server ──────────────────────────────────────────────────────────
const wss = new WebSocketServer({ server: http, maxPayload: SENSES_FRAME_BYTES + 64 })

wss.on('connection', (ws, req) => {
  // Extract real IP (CF tunnel adds CF-Connecting-IP header)
  const ip = req.headers['cf-connecting-ip']
           || req.headers['x-forwarded-for']?.split(',')[0]?.trim()
           || req.socket.remoteAddress
           || 'unknown'

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

  // Spawn body in headless mode
  const child = spawn(BODY_BIN, [runtimeDir, '--headless', socketPath], {
    stdio: 'ignore',
    detached: false,
  })

  const entry = {
    ws,
    process:    child,
    socketPath,
    runtimeDir,
    startedAt:  Date.now(),
  }
  activeSessions.set(sessionId, entry)
  idleTracker.add(sessionId)

  // Hard session timeout
  const hardTimeout = setTimeout(() => {
    ws.close(1001, 'session-limit')
  }, MAX_SESSION_MIN * 60_000)

  // Connect to body's Unix socket
  let bodySocket   = null
  let vocalBuf     = Buffer.alloc(0)
  let stateTimer   = null
  let badFrames    = 0
  const MAX_BAD    = 10  // disconnect after N consecutive bad frames

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
            if (ws.readyState === 1) ws.send(vocalBuf.slice(0, need))
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
      ws.send(frame)
    } catch { /* snapshot not ready */ }
  }, 1000)

  // Layer 4 + 5: validate every incoming frame; track idle
  ws.on('message', (data, isBinary) => {
    if (!isBinary) { ws.close(1003, 'text-not-allowed'); return }

    const buf = Buffer.isBuffer(data) ? data : Buffer.from(data)

    if (!validateFrame(buf)) {
      badFrames++
      if (badFrames >= MAX_BAD) ws.close(1008, 'bad-frames')
      return
    }
    badFrames = 0
    idleTracker.touch(sessionId)

    try { bodySocket?.write(buf) } catch { /* body socket closed */ }
  })

  const cleanup = () => {
    clearTimeout(hardTimeout)
    clearInterval(stateTimer)
    idleTracker.remove(sessionId)
    sessionGuard.release(sessionId)
    activeSessions.delete(sessionId)

    // Give body 2 s to write checkpoint, then terminate
    setTimeout(() => {
      child.kill('SIGTERM')
      setTimeout(() => {
        try { rmSync(runtimeDir, { recursive: true, force: true }) } catch { }
      }, 3000)
    }, 2000)
  }

  ws.on('close', cleanup)
  ws.on('error', () => ws.close())
  child.on('exit', () => {
    activeSessions.delete(sessionId)
    if (ws.readyState === 1) ws.close(1000)
  })
})

http.listen(PORT, '127.0.0.1', () => {
  // Bind to 127.0.0.1 only — CF tunnel is the only way in.
  // No direct internet access to this port.
  console.log(`[aborn-gateway] :${PORT} bound to 127.0.0.1 (tunnel only)`)
  console.log(`[aborn-gateway] body=${BODY_BIN} sessions=${SESSIONS_DIR}`)
})
