/**
 * aborn-gateway — session manager and sensory bridge.
 *
 * Each WebSocket connection = one body process with its own runtime dir.
 * Binary protocol (opaque — no field names, no JSON):
 *
 * Browser → gateway (sensory input, ~20 Hz):
 *   [0x01][audio_level:f32le][audio_flux:f32le][bands:12×f32le]
 *         [motion:f32le][luma:f32le][grid:768×f32le]
 *   Total: 3137 bytes
 *
 * Gateway → browser (vocal PCM, on demand):
 *   [0x02][n:u16le][samples:n×f32le]
 *
 * Gateway → browser (life-state pulse, ~1 Hz):
 *   [0x03][reserve:f32le][vital_state:u8][self_coherence:f32le][gap_pressure:f32le]
 *
 * Security:
 * - Raw video/audio never reaches the server (Perception.ts handles it locally).
 * - Binary frames carry no field names, no design info.
 * - Each session isolated in its own directory under /tmp/aborn-sessions/.
 * - Source code is NOT deployed here; only compiled binaries.
 */

import { createServer }    from 'node:http';
import { WebSocketServer } from 'ws';
import { createConnection } from 'node:net';
import { spawn }            from 'node:child_process';
import { readFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { join, resolve }    from 'node:path';
import { createReadStream } from 'node:fs';
import { extname }          from 'node:path';
import { createServer as createNetServer } from 'node:net';
import { createConnection as netConnect }  from 'node:net';

// ── Config ────────────────────────────────────────────────────────────────────
const PORT          = parseInt(process.env.ABORN_PORT   || '3900', 10);
const BODY_BIN      = process.env.ABORN_BODY_BIN || resolve('../bionic_v0/target/release/body');
const SESSIONS_DIR  = process.env.ABORN_SESSIONS || '/tmp/aborn-sessions';
const STATIC_DIR    = process.env.ABORN_STATIC   || resolve('../site/dist');
const MAX_SESSIONS  = parseInt(process.env.ABORN_MAX    || '8',    10);

mkdirSync(SESSIONS_DIR, { recursive: true });

// ── MIME types for static serving ────────────────────────────────────────────
const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js':   'application/javascript',
  '.css':  'text/css',
  '.svg':  'image/svg+xml',
  '.png':  'image/png',
  '.ico':  'image/x-icon',
  '.woff2':'font/woff2',
  '.json': 'application/json',
};

// ── Active sessions ───────────────────────────────────────────────────────────
const sessions = new Map(); // sessionId → { process, socketPath, runtimeDir, socket }

function newSessionId() {
  return `s${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

// ── HTTP server (static files + upgrade for WS) ───────────────────────────────
const http = createServer((req, res) => {
  let path = req.url.split('?')[0];
  if (path === '/' || !path.includes('.')) path = '/index.html';

  const file = join(STATIC_DIR, path);
  const ext  = extname(file);
  const mime = MIME[ext] || 'application/octet-stream';

  if (!existsSync(file)) {
    // SPA fallback
    const idx = join(STATIC_DIR, 'index.html');
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    createReadStream(idx).pipe(res);
    return;
  }

  res.writeHead(200, { 'Content-Type': mime });
  createReadStream(file).pipe(res);
});

// ── WebSocket server ──────────────────────────────────────────────────────────
const wss = new WebSocketServer({ server: http });

wss.on('connection', (ws, req) => {
  if (sessions.size >= MAX_SESSIONS) {
    ws.close(1013, 'capacity');
    return;
  }

  const sid        = newSessionId();
  const runtimeDir = join(SESSIONS_DIR, sid);
  const socketPath = join(runtimeDir, 'net.sock');
  mkdirSync(runtimeDir, { recursive: true });

  // Spawn body in headless mode.
  const child = spawn(BODY_BIN, [runtimeDir, '--headless', socketPath], {
    stdio: 'ignore',
    detached: false,
  });

  // Unix socket client: connect once body has created the socket (retry loop).
  let bodySocket = null;
  let vocal_buf  = Buffer.alloc(0);
  let stateTimer = null;

  const tryConnect = (attempts = 0) => {
    if (attempts > 40) { ws.close(1011, 'body timeout'); return; }
    if (!existsSync(socketPath)) {
      setTimeout(() => tryConnect(attempts + 1), 250);
      return;
    }
    const sock = createNetServer(); // we're not binding; use net.createConnection
    // Body is the server; gateway is the client.
    const c = netConnect(socketPath);
    c.on('error', () => setTimeout(() => tryConnect(attempts + 1), 250));
    c.once('connect', () => {
      bodySocket = c;
      sessions.get(sid).socket = c;

      // Forward vocal PCM + state pulses to the browser.
      c.on('data', (chunk) => {
        vocal_buf = Buffer.concat([vocal_buf, chunk]);
        while (vocal_buf.length >= 3) {
          const tag = vocal_buf[0];
          if (tag === 0x02) {
            if (vocal_buf.length < 3) break;
            const n = vocal_buf.readUInt16LE(1);
            const need = 3 + n * 4;
            if (vocal_buf.length < need) break;
            if (ws.readyState === 1 /* OPEN */) ws.send(vocal_buf.slice(0, need));
            vocal_buf = vocal_buf.slice(need);
          } else {
            // Unknown tag: discard one byte and resync.
            vocal_buf = vocal_buf.slice(1);
          }
        }
      });

      c.on('close', () => { if (ws.readyState === 1) ws.close(1000); });
    });
  };
  // Give body 300 ms to start before polling for the socket.
  setTimeout(() => tryConnect(), 300);

  sessions.set(sid, { process: child, socketPath, runtimeDir, socket: null });

  // Send periodic life-state pulses (0x03) by reading the snapshot.
  // We reuse the snapshot binary via the observe_tower-style read, but
  // for simplicity we just forward a compact fixed-format state blob.
  // The body saves snapshot periodically; we read it at 1 Hz.
  const SNAPSHOT_PATH = join(runtimeDir, 'snapshot.bin');
  stateTimer = setInterval(() => {
    if (ws.readyState !== 1) return;
    if (!existsSync(SNAPSHOT_PATH)) return;
    try {
      const snap = readFileSync(SNAPSHOT_PATH);
      // vital_core.bin is more reliable for this; read it if present.
      const vcPath = join(runtimeDir, 'vital_core.bin');
      if (!existsSync(vcPath)) return;
      const vc = readFileSync(vcPath);
      if (vc.length < 41) return;
      // vital_core.bin layout: magic(8)+ver(8)+reserve(f64)+anxiety(f64)+pain(f64)+state(u8)
      const reserve       = vc.readDoubleBE ? Number.NaN : vc.buffer
        ? new DataView(vc.buffer, vc.byteOffset, vc.byteLength).getFloat64(16, true)
        : 0;
      const vital_state   = vc[40];
      // Build a compact 0x03 state frame (10 bytes).
      const frame = Buffer.allocUnsafe(10);
      frame[0] = 0x03;
      frame.writeFloatLE(Math.min(1, Math.max(0, reserve)), 1);
      frame[5] = vital_state;
      frame.writeFloatLE(0, 6); // self_coherence placeholder
      ws.send(frame);
    } catch { /* snapshot not ready yet */ }
  }, 1000);

  // Receive sensory Senses frames from the browser and forward to body.
  ws.on('message', (data, isBinary) => {
    if (!isBinary || !bodySocket) return;
    try { bodySocket.write(data); } catch { /* body socket closed */ }
  });

  ws.on('close', () => {
    clearInterval(stateTimer);
    // Give body 2 s to write its checkpoint before killing it.
    setTimeout(() => {
      child.kill('SIGTERM');
      setTimeout(() => {
        try { rmSync(runtimeDir, { recursive: true, force: true }); } catch { }
      }, 3000);
    }, 2000);
    sessions.delete(sid);
  });

  ws.on('error', () => ws.close());
  child.on('exit', () => { sessions.delete(sid); });
});

http.listen(PORT, () => {
  console.log(`aborn-gateway listening on :${PORT}`);
  console.log(`body binary: ${BODY_BIN}`);
  console.log(`static:      ${STATIC_DIR}`);
  console.log(`sessions:    ${SESSIONS_DIR}`);
});
