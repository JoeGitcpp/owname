import type { Senses } from './perception'

// A quiescent sense bundle — no camera, no mic. Drives the ambient idle field
// behind the landing page: the substrate drifting, perceiving nothing yet.
export function idleSenses(): Senses {
  const gw = 32
  const gh = 24
  return {
    audioLevel: 0,
    audioFlux: 0,
    bands: new Float32Array(12),
    motion: 0,
    luma: 0.5,
    hue: 0.5,
    gw,
    gh,
    grid: new Float32Array(gw * gh).fill(0.5),
    hasVideo: false,
  }
}
