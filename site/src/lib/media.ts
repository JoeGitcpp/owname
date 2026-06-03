// Sensory channel acquisition. aborn perceives presence through camera + mic.
// We request both; they are consumed only client-side (see legal/privacy copy).

export type MediaResult =
  | { ok: true; stream: MediaStream }
  | { ok: false; reason: 'denied' | 'unsupported' | 'error'; detail?: string }

export function mediaSupported(): boolean {
  return (
    typeof navigator !== 'undefined' &&
    !!navigator.mediaDevices &&
    typeof navigator.mediaDevices.getUserMedia === 'function'
  )
}

export async function acquireSenses(): Promise<MediaResult> {
  if (!mediaSupported()) return { ok: false, reason: 'unsupported' }
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 } },
      audio: { echoCancellation: false, noiseSuppression: false },
    })
    return { ok: true, stream }
  } catch (err) {
    const e = err as DOMException
    if (e && (e.name === 'NotAllowedError' || e.name === 'SecurityError')) {
      return { ok: false, reason: 'denied', detail: e.message }
    }
    if (e && (e.name === 'NotFoundError' || e.name === 'OverconstrainedError')) {
      // No camera/mic present — treat as unsupported environment.
      return { ok: false, reason: 'unsupported', detail: e.message }
    }
    return { ok: false, reason: 'error', detail: e?.message ?? String(err) }
  }
}

export function releaseStream(stream: MediaStream | null) {
  if (!stream) return
  for (const track of stream.getTracks()) track.stop()
}
