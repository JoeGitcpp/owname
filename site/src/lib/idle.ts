import type { Senses } from './perception'

// A quiescent sense bundle — no camera, no mic. Drives the ambient idle field
// behind the landing page: the substrate drifting, perceiving nothing yet.
export function idleSenses(): Senses {
  const gw = 160
  const gh = 120
  const n = gw * gh
  return {
    audioLevel: 0,
    audioFlux: 0,
    bands: new Float32Array(64),
    motion: 0,
    luma: 0.5,
    hue: 0.5,
    gw,
    gh,
    grid: new Float32Array(n).fill(0.5),
    gridR: new Float32Array(n).fill(0.5),
    gridG: new Float32Array(n).fill(0.5),
    gridB: new Float32Array(n).fill(0.5),
    spectralCentroid: 0,
    spectralFlatness: 0,
    spectralRolloff: 0,
    spectralSpread: 0,
    zcr: 0,
    hnr: 0.5,
    onsetStrength: 0,
    temporalFlux: 0,
    hasVideo: false,
  }
}
