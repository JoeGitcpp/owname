// Modality — the outward face of the integrating substrate. It does not draw
// "a face" or "a chat". It draws a field of motion that flows along the
// contours the camera perceives and breathes with the sound it hears, while a
// small set of tier-0 readouts (drive / err / precision / re-entry) expose the
// internal integration that produces it.

import type { Senses } from './perception'

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  life: number
  max: number
}

// Internal, slowly-integrating state — the substrate's "how it is right now".
interface Inner {
  drive: number // overall activation
  err: number // prediction error (surprise)
  precision: number // confidence / order
  reentry: number // recurrent self-excitation
  hue: number
  coreR: number
}

export class Modality {
  private cv: HTMLCanvasElement
  private ctx: CanvasRenderingContext2D
  private dpr = 1
  private w = 0
  private h = 0

  private particles: Particle[] = []
  private pred: Float32Array | null = null // predicted luminance grid
  private inner: Inner = {
    drive: 0,
    err: 0,
    precision: 0.5,
    reentry: 0,
    hue: 0.5,
    coreR: 0,
  }
  private t = 0
  private errHistory: number[] = []
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
    // Scale population to screen area, capped for low-power devices.
    const target = Math.min(1400, Math.floor((this.w * this.h) / 1100))
    const arr: Particle[] = []
    for (let i = 0; i < target; i++) arr.push(this.spawn())
    this.particles = arr
  }

  private spawn(): Particle {
    const max = 80 + ((this.t * 97 + this.particles.length * 131) % 160)
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

  // bilinear sample of the luminance grid in screen space → [0,1]
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
    // prediction error: compare current grid to our running prediction
    if (!this.pred || this.pred.length !== s.grid.length) {
      this.pred = new Float32Array(s.grid)
    }
    const pred = this.pred
    let err = 0
    for (let i = 0; i < s.grid.length; i++) {
      err += Math.abs(s.grid[i] - pred[i])
      pred[i] += (s.grid[i] - pred[i]) * 0.06 // assimilate slowly
    }
    err = clamp01((err / s.grid.length) * 7)

    const I = this.inner
    const target = clamp01(s.audioLevel * 0.7 + s.motion * 0.7 + s.audioFlux * 0.4)
    I.drive += (target - I.drive) * 0.08
    I.err += (err - I.err) * 0.12
    // precision rises when error is low and there is signal to be confident about
    const conf = clamp01(1 - I.err * 1.6) * clamp01(0.2 + I.drive)
    I.precision += (conf - I.precision) * 0.05
    // re-entry: self-excitation that lingers after stimulation
    I.reentry += (I.drive - I.reentry) * 0.03
    I.reentry *= 0.995
    I.hue = s.hasVideo ? s.hue : (I.hue + 0.0004) % 1
    const coreTarget = Math.min(this.w, this.h) * (0.10 + I.drive * 0.16 + I.reentry * 0.08)
    I.coreR += (coreTarget - I.coreR) * 0.1

    this.errHistory.push(I.err)
    if (this.errHistory.length > 96) this.errHistory.shift()
  }

  private frame() {
    this.t += 1
    const s = this.readSenses()
    this.updateInner(s)
    const ctx = this.ctx
    const I = this.inner

    // trail fade — lower alpha = longer ghosting; agitation shortens trails
    const fade = 0.07 + I.err * 0.12
    ctx.globalCompositeOperation = 'source-over'
    ctx.fillStyle = `rgba(5, 7, 10, ${fade})`
    ctx.fillRect(0, 0, this.w, this.h)

    const cx = this.w / 2
    const cy = this.h / 2

    // ---- particle field (additive glow) ----
    ctx.globalCompositeOperation = 'lighter'
    const speed = 0.6 + I.drive * 2.6
    const turb = 0.4 + s.motion * 2.2
    const eps = 6
    const baseHue = I.hue
    for (const p of this.particles) {
      // contour following: gradient of perceived luminance, then steer 90°
      let ax = 0
      let ay = 0
      if (s.hasVideo) {
        const gx =
          this.sampleGrid(s, clampX(p.x + eps, this.w), p.y) -
          this.sampleGrid(s, clampX(p.x - eps, this.w), p.y)
        const gy =
          this.sampleGrid(s, p.x, clampY(p.y + eps, this.h)) -
          this.sampleGrid(s, p.x, clampY(p.y - eps, this.h))
        // perpendicular to the gradient → flow along edges of the figure
        ax += -gy * 40
        ay += gx * 40
      }
      // organic curl so empty regions still drift
      const n = this.t * 0.006
      ax += Math.cos(p.y * 0.012 + n + Math.sin(p.x * 0.008)) * turb
      ay += Math.sin(p.x * 0.012 - n + Math.cos(p.y * 0.008)) * turb
      // gentle pull toward the core, modulated by re-entry
      const dx = cx - p.x
      const dy = cy - p.y
      const dist = Math.hypot(dx, dy) + 0.001
      const pull = (0.02 + I.reentry * 0.05) * (dist > I.coreR ? 1 : -0.6)
      ax += (dx / dist) * pull * 30
      ay += (dy / dist) * pull * 30

      p.vx = p.vx * 0.86 + ax * 0.02 * speed
      p.vy = p.vy * 0.86 + ay * 0.02 * speed
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
      const alpha = 0.05 + lifeT * 0.35 * (0.4 + I.precision * 0.9)
      const speedMag = Math.min(1, Math.hypot(p.vx, p.vy) / 5)
      // hue drifts per-particle toward warm with local brightness & speed
      const hue = (baseHue + lum * 0.12 + speedMag * 0.06) % 1
      const [r, g, b] = hsl(hue, 0.55 + I.drive * 0.25, 0.45 + lum * 0.35)
      const size = 0.6 + speedMag * 1.6 + lum * 1.2
      ctx.fillStyle = `rgba(${r},${g},${b},${alpha})`
      ctx.beginPath()
      ctx.arc(p.x, p.y, size, 0, Math.PI * 2)
      ctx.fill()
    }

    // ---- presence core: breathing concentric rings + band harmonics ----
    const coreHue = (baseHue + 0.5) % 1 // complementary, so the core reads as "self"
    const [cr, cg, cb] = hsl(coreHue, 0.5, 0.6)
    const breath = 0.85 + Math.sin(this.t * 0.03) * 0.15
    for (let ring = 0; ring < 5; ring++) {
      const rr = I.coreR * (0.35 + ring * 0.22) * breath
      const a = (0.16 - ring * 0.022) * (0.5 + I.precision * 0.8)
      ctx.strokeStyle = `rgba(${cr},${cg},${cb},${a})`
      ctx.lineWidth = 1 + I.drive * 1.5
      ctx.beginPath()
      const steps = 96
      for (let i = 0; i <= steps; i++) {
        const ang = (i / steps) * Math.PI * 2
        // band harmonics deform the ring; audio sculpts the silhouette
        const bi = (ring + i) % s.bands.length
        const wob =
          1 +
          s.bands[bi] * 0.28 * Math.sin(ang * (2 + ring) + this.t * 0.05) +
          s.audioFlux * 0.12 * Math.sin(ang * 5 - this.t * 0.08)
        const x = cx + Math.cos(ang) * rr * wob
        const y = cy + Math.sin(ang) * rr * wob
        if (i === 0) ctx.moveTo(x, y)
        else ctx.lineTo(x, y)
      }
      ctx.stroke()
    }
    // core glow
    const glow = ctx.createRadialGradient(cx, cy, 0, cx, cy, I.coreR * 1.2)
    glow.addColorStop(0, `rgba(${cr},${cg},${cb},${0.10 + I.drive * 0.12})`)
    glow.addColorStop(1, 'rgba(0,0,0,0)')
    ctx.fillStyle = glow
    ctx.fillRect(cx - I.coreR * 1.3, cy - I.coreR * 1.3, I.coreR * 2.6, I.coreR * 2.6)

    // ---- vignette ----
    ctx.globalCompositeOperation = 'source-over'
    const vig = ctx.createRadialGradient(
      cx,
      cy,
      Math.min(this.w, this.h) * 0.3,
      cx,
      cy,
      Math.max(this.w, this.h) * 0.75,
    )
    vig.addColorStop(0, 'rgba(0,0,0,0)')
    vig.addColorStop(1, 'rgba(0,0,0,0.55)')
    ctx.fillStyle = vig
    ctx.fillRect(0, 0, this.w, this.h)

    if (this.showReadout) this.drawReadout(s)
  }

  // tier-0 readout — the substrate, observed. Mono text + value bars.
  private drawReadout(s: Senses) {
    const ctx = this.ctx
    const I = this.inner
    ctx.globalCompositeOperation = 'source-over'
    // opaque backing strip so changing values don't ghost through the trail fade
    ctx.fillStyle = 'rgba(5, 7, 10, 0.96)'
    ctx.fillRect(12, this.h - 104, 290, 100)
    ctx.font = '11px ui-monospace, SFMono-Regular, Menlo, monospace'
    ctx.textBaseline = 'top'
    const x = 20
    let y = this.h - 96
    const rows: [string, number][] = [
      ['drive', I.drive],
      ['err', I.err],
      ['precision', I.precision],
      ['re-entry', clamp01(I.reentry)],
    ]
    for (const [label, val] of rows) {
      ctx.fillStyle = 'rgba(138,151,166,0.6)'
      ctx.fillText(label.padEnd(10), x, y)
      // bar
      const bx = x + 86
      const bw = 120
      ctx.fillStyle = 'rgba(26,34,46,0.9)'
      ctx.fillRect(bx, y + 2, bw, 6)
      ctx.fillStyle = 'rgba(95,211,196,0.85)'
      ctx.fillRect(bx, y + 2, bw * clamp01(val), 6)
      ctx.fillStyle = 'rgba(138,151,166,0.5)'
      ctx.fillText(val.toFixed(3), bx + bw + 10, y)
      y += 18
    }
    // tiny figure-perception tell
    if (!s.hasVideo) {
      ctx.fillStyle = 'rgba(217,160,91,0.7)'
      ctx.fillText('· no visual channel ·', x, y + 2)
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
// deterministic pseudo-random in [0,1) — avoids Math.random for reproducibility
function pseudo(seed: number) {
  const x = Math.sin(seed * 12.9898) * 43758.5453
  return x - Math.floor(x)
}
// hsl (h,s,l in 0..1) → [r,g,b] 0..255
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
