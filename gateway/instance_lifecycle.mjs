const TRANSITIONS = {
  pending_verification: { verify: 'running',    destroy: 'destroyed' },
  running:              { suspend: 'suspended', destroy: 'destroyed' },
  suspended:            { resume: 'running',    destroy: 'destroyed' },
  destroyed:            {},
}

export const STATES = Object.keys(TRANSITIONS)

export function transition(state, event) {
  if (!(state in TRANSITIONS)) throw new Error(`unknown state: ${state}`)
  const next = TRANSITIONS[state][event]
  if (!next) throw new Error(`invalid transition: ${state} + ${event}`)
  return next
}
