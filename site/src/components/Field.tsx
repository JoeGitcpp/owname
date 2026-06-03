import { useEffect, useRef } from 'react'
import { Modality } from '../lib/modality'
import type { Senses } from '../lib/perception'

// Mounts a full-bleed canvas and runs the Modality renderer against whatever
// sense source is provided. Used both for the ambient landing field (idle
// senses) and the live communion viewport (camera + mic senses).
export function Field({
  read,
  className,
  readout = true,
}: {
  read: () => Senses
  className?: string
  readout?: boolean
}) {
  const ref = useRef<HTMLCanvasElement>(null)
  const readRef = useRef(read)
  readRef.current = read

  useEffect(() => {
    const cv = ref.current
    if (!cv) return
    const modality = new Modality(cv, () => readRef.current(), { readout })
    const onResize = () => modality.resize()
    window.addEventListener('resize', onResize)
    modality.start()
    return () => {
      window.removeEventListener('resize', onResize)
      modality.stop()
    }
  }, [readout])

  return <canvas ref={ref} className={className} aria-hidden="true" />
}
