// Modality — the outward face of the integrating substrate.
// Streaming / hardcore mode: scrolling signal history, live waveform ring,
// velocity-streak particles, and spectrum bars. No static circles.

import type { Senses } from './perception'

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  life: number
  max: number
}

interface SignalSnap {
  drive: number
  err: number
  precision: number
  reentry: number
}

// Internal, slowly-integrating state — the substrate's "how it is right now".
interface Inner {
  drive: number
  err: number
  precision: number
  reentry: number
  hue: number
  coreR: number
}

const HIST_LEN = 260

export class Modality {
  private cv: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private dpr = 1
  private w = 0
  private h = 0

  private particles: Particle[] = []
  private pred: Float32Array | null = null
  private inner: Inner = {
    drive: 0,
    err: 0,
    precision: 0.5,
    reentry: 0,
    hue: 0.5,
    coreR: 0,
  }
  private t = 0
  private history: SignalSnap[] = []
  private raf = 0
  private readSenses: () => Senses
  private running = false
  private showReadout: boolean

  constructor(
    canvas: HTMLCanvasElement,
    readSenses: () => Senses,
    opts: { readout?: boolean } = {},
  ) {
    this.cv = canvas
    const ctx = canvas.getContext('2d', { alpha: false })
    if (!ctx) throw new Error('2d context unavailable')
    this.ctx = ctx
    this.readSenses = readSenses
    this.showReadout = opts.readout ?? true
    this.resize()
  }

  resize() {
    this.dpr = Math.min(window.devicePixelRatio || 1, 2)
    const w = this.cv.clientWidth || window.innerWidth
    const h = this.cv.clientHeight || window.innerHeight
    this.w = w
    this.h = h
    this.cv.width = Math.floor(w * this.dpr)
    this.cv.height = Math.floor(h * this.dpr)
    this.ctx.setTransform(this.dpr, 0, 0, this.dpr, 0, 0)
    this.seedParticles()
  }

  private seedParticles() {
    // Fewer, larger particles — more intentional, less noisy-dot feel
    const target = Math.min(420, Math.floor((this.w * this.h) / 5000))
    const arr: Particle[] = []
    for (let i = 0; i < target; i++) arr.push(this.spawn())
    this.particles = arr
  }

  private spawn(): Particle {
    const max = 120 + ((this.t * 97 + this.particles.length * 131) % 200)
    return {
      x: pseudo(this.t + this.particles.length * 1.7) * this.w,
      y: pseudo(this.t * 1.3 + this.particles.length * 2.9) * this.h,
      vx: 0,
      vy: 0,
      life: pseudo(this.t + this.particles.length) * max,
      max,
    }
  }

  start() {
    if (this.running) return
    this.running = true
    const loop = () => {
      if (!this.running) return
      this.frame()
      this.raf = requestAnimationFrame(loop)
    }
    this.raf = requestAnimationFrame(loop)
  }

  stop() {
    this.running = false
    if (this.raf) cancelAnimationFrame(this.raf)
  }

  private sampleGrid(s: Senses, x: number, y: number): number {
    const gx = (x / this.w) * (s.gw - 1)
    const gy = (y / this.h) * (s.gh - 1)
    const x0 = Math.floor(gx)
    const y0 = Math.floor(gy)
    const x1 = Math.min(s.gw - 1, x0 + 1)
    const y1 = Math.min(s.gh - 1, y0 + 1)
    const fx = gx - x0
    const fy = gy - y0
    const g = s.grid
    const a = g[y0 * s.gw + x0]
    const b = g[y0 * s.gw + x1]
    const c = g[y1 * s.gw + x0]
    const d = g[y1 * s.gw + x1]
    return a * (1 - fx) * (1 - fy) + b * fx * (1 - fy) + c * (1 - fx) * fy + d * fx * fy
  }

  private updateInner(s: Senses) {
    if (!this.pred || this.pred.length !== s.grid.length) {
      this.pred = new Float32Array(s.grid)
    }
    const pred = this.pred
    let err = 0
    for (let i = 0; i < s.grid.length; i++) {
      err += Math.abs(s.grid[i] - pred[i])
      pred[i] += (s.grid[i] - pred[i]) * 0.06
    }
    err = clamp01((err / s.grid.length) * 7)

    const I = this.inner
    const target = clamp01(s.audioLevel * 0.7 + s.motion * 0.7 + s.audioFlux * 0.4)
    I.drive += (target - I.drive) * 0.08
    I.err += (err - I.err) * 0.12
    const conf = clamp01(1 - I.err * 1.6) * clamp01(0.2 + I.drive)
    I.precision += (conf - I.precision) * 0.05
    I.reentry += (I.drive - I.reentry) * 0.03
    I.reentry *= 0.995
    I.hue = s.hasVideo ? s.hue : (I.hue + 0.0004) % 1
    const coreTarget = Math.min(this.w, this.h) * (0.10 + I.drive * 0.16 + I.reentry * 0.08)
    I.coreR += (coreTarget - I.coreR) * 0.1

    // Append to scrolling signal history
    this.history.push({
      drive: I.drive,
      err: I.err,
      precision: I.precision,
      reentry: clamp01(I.reentry),
    })
    if (this.history.length > HIST_LEN) this.history.shift()
  }

  private frame() {
    this.t += 1
    const s = this.readSenses()
    this.updateInner(s)
    const ctx = this.ctx
    const I = this.inner

    // trail fade
    const fade = 0.09 + I.err * 0.10
    ctx.globalCompositeOperation = 'source-over'
    ctx.fillStyle = `rgba(4, 6, 10, ${fade})`
    ctx.fillRect(0, 0, this.w, this.h)

    const cx = this.w / 2
    const cy = this.h / 2

    // ---- velocity-streak particle field (additive) ----
    ctx.globalCompositeOperation = 'lighter'
    const speed = 0.7 + I.drive * 2.8
    const turb = 0.5 + s.motion * 2.4
    const eps = 6
    const baseHue = I.hue

    for (const p of this.particles) {
      let ax = 0
      let ay = 0
      if (s.hasVideo) {
        const gx =
          this.sampleGrid(s, clampX(p.x + eps, this.w), p.y) -
          this.sampleGrid(s, clampX(p.x - eps, this.w), p.y)
        const gy =
          this.sampleGrid(s, p.x, clampY(p.y + eps, this.h)) -
          this.sampleGrid(s, p.x, clampY(p.y - eps, this.h))
        ax += -gy * 38
        ay += gx * 38
      }
      const n = this.t * 0.006
      ax += Math.cos(p.y * 0.012 + n + Math.sin(p.x * 0.008)) * turb
      ay += Math.sin(p.x * 0.012 - n + Math.cos(p.y * 0.008)) * turb
      const dx = cx - p.x
      const dy = cy - p.y
      const dist = Math.hypot(dx, dy) + 0.001
      const pull = (0.02 + I.reentry * 0.05) * (dist > I.coreR ? 1 : -0.6)
      ax += (dx / dist) * pull * 30
      ay += (dy / dist) * pull * 30

      p.vx = p.vx * 0.84 + ax * 0.022 * speed
      p.vy = p.vy * 0.84 + ay * 0.022 * speed
      p.x += p.vx
      p.y += p.vy
      p.life -= 1

      if (
        p.life <= 0 ||
        p.x < -20 ||
        p.x > this.w + 20 ||
        p.y < -20 ||
        p.y > this.h + 20
      ) {
        Object.assign(p, this.spawn())
        continue
      }

      const lum = s.hasVideo ? this.sampleGrid(s, p.x, p.y) : 0.5
      const lifeT = p.life / p.max
      const alpha = 0.06 + lifeT * 0.38 * (0.4 + I.precision * 0.9)
      const speedMag = Math.min(1, Math.hypot(p.vx, p.vy) / 5)
      const hue = (baseHue + lum * 0.12 + speedMag * 0.06) % 1
      const [r, g, b] = hsl(hue, 0.55 + I.drive * 0.25, 0.45 + lum * 0.35)

      // Draw as velocity streak instead of dot — looks intentional, not noisy
      const sv = Math.hypot(p.vx, p.vy)
      if (sv > 0.5) {
        ctx.strokeStyle = `rgba(${r},${g},${b},${alpha * 0.9})`
        ctx.lineWidth = 1.2 + lum * 1.0
        ctx.beginPath()
        const trailLen = Math.min(4, sv * 0.8)
        ctx.moveTo(p.x - (p.vx / sv) * trailLen, p.y - (p.vy / sv) * trailLen)
        ctx.lineTo(p.x, p.y)
        ctx.stroke()
      } else {
        ctx.fillStyle = `rgba(${r},${g},${b},${alpha})`
        ctx.beginPath()
        ctx.arc(p.x, p.y, 1.4 + lum * 0.9, 0, Math.PI * 2)
        ctx.fill()
      }
    }

    // ---- live waveform ring — single dynamic shape (replaces 5 static rings) ----
    ctx.globalCompositeOperation = 'lighter'
    const coreHue = (baseHue + 0.5) % 1
    const [cr, cg, cb] = hsl(coreHue, 0.55, 0.65)
    const ringR = I.coreR * (0.88 + I.drive * 0.14)
    ctx.strokeStyle = `rgba(${cr},${cg},${cb},${0.55 + I.precision * 0.35})`
    ctx.lineWidth = 1.4 + I.drive * 2.2
    ctx.beginPath()
    const RING_STEPS = 160
    for (let i = 0; i <= RING_STEPS; i++) {
      const ang = (i / RING_STEPS) * Math.PI * 2
      const bi = Math.floor((i / RING_STEPS) * s.bands.length) % s.bands.length
      const bandV = s.bands[bi]
      const deform =
        1 +
        bandV * 0.42 * Math.sin(ang * 5 + this.t * 0.05) +
        s.audioFlux * 0.18 * Math.sin(ang * 11 - this.t * 0.08) +
        I.err * 0.12 * Math.sin(ang * 3 + this.t * 0.03)
      const r = ringR * deform
      const x = cx + Math.cos(ang) * r
      const y = cy + Math.sin(ang) * r
      if (i === 0) ctx.moveTo(x, y)
      else ctx.lineTo(x, y)
    }
    ctx.closePath()
    ctx.stroke()

    // core glow
    const glow = ctx.createRadialGradient(cx, cy, 0, cx, cy, I.coreR * 1.2)
    glow.addColorStop(0, `rgba(${cr},${cg},${cb},${0.09 + I.drive * 0.11})`)
    glow.addColorStop(1, 'rgba(0,0,0,0)')
    ctx.fillStyle = glow
    ctx.fillRect(cx - I.coreR * 1.3, cy - I.coreR * 1.3, I.coreR * 2.6, I.coreR * 2.6)

    // ---- vignette ----
    ctx.globalCompositeOperation = 'source-over'
    const vig = ctx.createRadialGradient(
      cx, cy, Math.min(this.w, this.h) * 0.3,
      cx, cy, Math.max(this.w, this.h) * 0.75,
    )
    vig.addColorStop(0, 'rgba(0,0,0,0)')
    vig.addColorStop(1, 'rgba(0,0,0,0.55)')
    ctx.fillStyle = vig
    ctx.fillRect(0, 0, this.w, this.h)

    // ---- streaming signal strip (top) ----
    this.drawSignalStrip(s)

    // ---- spectrum bars (bottom) ----
    this.drawSpectrumBars(s)

    if (this.showReadout) this.drawReadout(s)
  }

  // Scrolling 4-channel signal history — EKG / oscilloscope style
  private drawSignalStrip(s: Senses) {
    const ctx = this.ctx
    const W = this.w
    const stripH = 68
    const stripY = 10
    const stripX = 12
    const stripW = W - 24

    ctx.globalCompositeOperation = 'source-over'
    ctx.fillStyle = 'rgba(4, 7, 12, 0.88)'
    ctx.fillRect(stripX, stripY, stripW, stripH)

    // subtle outer border
    ctx.strokeStyle = 'rgba(40, 60, 90, 0.7)'
    ctx.lineWidth = 0.5
    ctx.strokeRect(stripX + 0.5, stripY + 0.5, stripW - 1, stripH - 1)

    type SignalKey = keyof SignalSnap
    const signals: [SignalKey, string, string][] = [
      ['drive',     'rgba(70, 210, 185, 0.9)',  'drv'],
      ['err',       'rgba(255, 135, 70, 0.9)',  'err'],
      ['precision', 'rgba(100, 155, 255, 0.9)', 'prc'],
      ['reentry',   'rgba(195, 110, 255, 0.85)','rnt'],
    ]
    const laneH = stripH / signals.length
    const hist = this.history

    for (let si = 0; si < signals.length; si++) {
      const [key, color, label] = signals[si]
      const laneY = stripY + si * laneH

      // lane divider
      if (si > 0) {
        ctx.strokeStyle = 'rgba(30, 50, 75, 0.8)'
        ctx.lineWidth = 0.5
        ctx.beginPath()
        ctx.moveTo(stripX, laneY)
        ctx.lineTo(stripX + stripW, laneY)
        ctx.stroke()
      }

      // signal line
      ctx.strokeStyle = color
      ctx.lineWidth = 1.1
      ctx.beginPath()
      for (let i = 0; i < hist.length; i++) {
        const x = stripX + (i / HIST_LEN) * stripW
        const val = hist[i][key] as number
        const y = laneY + laneH - 1 - val * (laneH - 3)
        if (i === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      ctx.stroke()

      // label (left edge)
      ctx.fillStyle = color.replace('0.9', '0.55').replace('0.85', '0.55')
      ctx.font = '9px ui-monospace, SFMono-Regular, Menlo, monospace'
      ctx.textBaseline = 'middle'
      ctx.fillText(label, stripX + 3, laneY + laneH / 2)

      // current value (right edge)
      const cur = hist.length > 0 ? (hist[hist.length - 1][key] as number) : 0
      ctx.fillStyle = color
      ctx.textAlign = 'right'
      ctx.fillText(cur.toFixed(2), stripX + stripW - 3, laneY + laneH / 2)
      ctx.textAlign = 'left'
    }

    // live cursor: vertical line at rightmost data point
    if (hist.length > 0) {
      const curX = stripX + (hist.length / HIST_LEN) * stripW
      ctx.strokeStyle = 'rgba(200, 220, 255, 0.25)'
      ctx.lineWidth = 1
      ctx.setLineDash([2, 3])
      ctx.beginPath()
      ctx.moveTo(curX, stripY)
      ctx.lineTo(curX, stripY + stripH)
      ctx.stroke()
      ctx.setLineDash([])
    }

    // audio-level micro-bar at top-right of strip
    const lvl = s.audioLevel
    const lvlW = Math.min(stripW * 0.18, 60)
    const lvlX = stripX + stripW - lvlW - 4
    const lvlY = stripY + 3
    ctx.fillStyle = 'rgba(20, 35, 55, 0.9)'
    ctx.fillRect(lvlX, lvlY, lvlW, 5)
    const [lr, lg, lb] = hsl(0.45 - lvl * 0.35, 0.8, 0.5)
    ctx.fillStyle = `rgba(${lr},${lg},${lb},0.95)`
    ctx.fillRect(lvlX, lvlY, lvlW * lvl, 5)
  }

  // Vertical frequency-band bars
  private drawSpectrumBars(s: Senses) {
    const ctx = this.ctx
    const W = this.w
    const H = this.h
    const BAR_H = 44
    const BAR_Y = H - BAR_H - 10
    const BAR_X = 12
    const BAR_W = W - 24
    const N = s.bands.length

    ctx.globalCompositeOperation = 'source-over'
    ctx.fillStyle = 'rgba(4, 7, 12, 0.88)'
    ctx.fillRect(BAR_X, BAR_Y - 4, BAR_W, BAR_H + 8)
    ctx.strokeStyle = 'rgba(40, 60, 90, 0.7)'
    ctx.lineWidth = 0.5
    ctx.strokeRect(BAR_X + 0.5, BAR_Y - 3.5, BAR_W - 1, BAR_H + 7)

    const cellW = BAR_W / N
    for (let b = 0; b < N; b++) {
      const v = s.bands[b]
      const bh = v * BAR_H
      const bx = BAR_X + b * cellW
      const by = BAR_Y + BAR_H - bh
      // hue: teal at low freq → warm at high
      const hue = 0.52 - (b / N) * 0.38
      const [r, g, bl] = hsl(hue, 0.75, 0.35 + v * 0.35)
      ctx.fillStyle = `rgba(${r},${g},${bl},0.92)`
      ctx.fillRect(bx + 1.5, by, cellW - 3, bh)

      // peak dot
      if (v > 0.08) {
        ctx.fillStyle = `rgba(${r},${g},${bl},0.55)`
        ctx.fillRect(bx + 1.5, by - 2, cellW - 3, 2)
      }
    }

    // label
    ctx.fillStyle = 'rgba(70, 100, 135, 0.7)'
    ctx.font = '8px ui-monospace, SFMono-Regular, Menlo, monospace'
    ctx.textBaseline = 'bottom'
    ctx.fillText('spectrum', BAR_X + 3, BAR_Y - 4)
    ctx.textBaseline = 'top'
  }

  // Tier-0 readout — right side, expanded
  private drawReadout(s: Senses) {
    const ctx = this.ctx
    const I = this.inner
    ctx.globalCompositeOperation = 'source-over'

    const RW = 200
    const RH = 92
    const RX = this.w - RW - 12
    const RY = this.h - 120  // sits above the spectrum bar area

    ctx.fillStyle = 'rgba(4, 7, 12, 0.92)'
    ctx.fillRect(RX, RY, RW, RH)
    ctx.strokeStyle = 'rgba(40, 60, 90, 0.7)'
    ctx.lineWidth = 0.5
    ctx.strokeRect(RX + 0.5, RY + 0.5, RW - 1, RH - 1)

    ctx.font = '10px ui-monospace, SFMono-Regular, Menlo, monospace'
    ctx.textBaseline = 'top'
    const x = RX + 8
    let y = RY + 7
    const BW = 82

    const rows: [string, number, string][] = [
      ['drive    ', I.drive,              'rgba(70,210,185,0.9)'],
      ['err      ', I.err,               'rgba(255,135,70,0.9)'],
      ['precision', I.precision,          'rgba(100,155,255,0.9)'],
      ['re-entry ', clamp01(I.reentry),  'rgba(195,110,255,0.85)'],
      ['audioLvl ', s.audioLevel,        'rgba(150,180,220,0.75)'],
    ]

    for (const [label, val, color] of rows) {
      ctx.fillStyle = 'rgba(80, 110, 145, 0.6)'
      ctx.fillText(label, x, y)
      const bx = x + 72
      ctx.fillStyle = 'rgba(18, 28, 40, 0.9)'
      ctx.fillRect(bx, y + 2, BW, 5)
      ctx.fillStyle = color
      ctx.fillRect(bx, y + 2, BW * clamp01(val), 5)
      ctx.fillStyle = color.replace('0.9', '0.7').replace('0.85', '0.7').replace('0.75', '0.7')
      ctx.fillText(val.toFixed(3), bx + BW + 5, y)
      y += 16
    }

    if (!s.hasVideo) {
      ctx.fillStyle = 'rgba(200, 150, 80, 0.65)'
      ctx.fillText('· no visual ·', x, y + 2)
    }
  }
}

// ---------- helpers ----------
function clamp01(x: number) {
  return x < 0 ? 0 : x > 1 ? 1 : x
}
function clampX(x: number, w: number) {
  return x < 0 ? 0 : x > w ? w : x
}
function clampY(y: number, h: number) {
  return y < 0 ? 0 : y > h ? h : y
}
function pseudo(seed: number) {
  const x = Math.sin(seed * 12.9898) * 43758.5453
  return x - Math.floor(x)
}
function hsl(h: number, s: number, l: number): [number, number, number] {
  if (s === 0) {
    const v = Math.round(l * 255)
    return [v, v, v]
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s
  const p = 2 * l - q
  const hk = h
  const tc = [hk + 1 / 3, hk, hk - 1 / 3].map((t) => {
    if (t < 0) t += 1
    if (t > 1) t -= 1
    if (t < 1 / 6) return p + (q - p) * 6 * t
    if (t < 1 / 2) return q
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6
    return p
  })
  return [Math.round(tc[0] * 255), Math.round(tc[1] * 255), Math.round(tc[2] * 255)]
}
