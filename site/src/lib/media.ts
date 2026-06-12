// Sensory channel acquisition. aborn perceives presence through camera + mic.
// We request both; they are consumed only client-side (see legal/privacy copy).

export type MediaResult =
  | { ok: true; stream: MediaStream; video: boolean }
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
  const audioConstraint = { echoCancellation: false, noiseSuppression: false }
  try {
    const stream = await navigator.mediaDevices.getUserMedia({
      video: { facingMode: 'user', width: { ideal: 640 }, height: { ideal: 480 } },
      audio: audioConstraint,
    })
    return { ok: true, stream, video: true }
  } catch (err) {
    const e = err as DOMException
    // Graceful degrade: camera missing or refused shouldn't kill the session —
    // aborn can still HEAR. Fall back to audio-only (the visual surface stays
    // dark; Commune shows a "turn on camera" hint). Only a full audio denial
    // ends the attempt.
    if (e && (e.name === 'NotAllowedError' || e.name === 'SecurityError' ||
              e.name === 'NotFoundError' || e.name === 'OverconstrainedError')) {
      try {
        const audioOnly = await navigator.mediaDevices.getUserMedia({ audio: audioConstraint })
        return { ok: true, stream: audioOnly, video: false }
      } catch (err2) {
        const e2 = err2 as DOMException
        if (e2 && (e2.name === 'NotAllowedError' || e2.name === 'SecurityError')) {
          return { ok: false, reason: 'denied', detail: e2.message }
        }
        if (e2 && (e2.name === 'NotFoundError' || e2.name === 'OverconstrainedError')) {
          return { ok: false, reason: 'unsupported', detail: e2.message }
        }
        return { ok: false, reason: 'error', detail: e2?.message ?? String(err2) }
      }
    }
    return { ok: false, reason: 'error', detail: e?.message ?? String(err) }
  }
}

export function releaseStream(stream: MediaStream | null) {
  if (!stream) return
  for (const track of stream.getTracks()) track.stop()
}
