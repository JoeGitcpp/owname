# Binary Character Organ v0 Design

Date: 2026-05-19
Status: LOCKED
Scope: `bionic_v0` body-level binary character ingress and egress organs

## Purpose

Binary Character Organ v0 promotes computer-native character I/O from loose
external protocol into a first-class body organ pair, at the same architectural
level as PHOT camera-like ingress, VIB microphone-like ingress, and vocal /
action output.

The organ pair is:

- `BinaryCharacterEar`: body-side sensory organ for concurrent binary
  character disturbance streams.
- `BinaryCharacterMouth`: body-side motor organ for binary character output
  bursts.

The virtual life room may provide virtual hardware for these organs, just as it
can provide virtual camera or microphone providers. The organ logic itself
belongs inside `bionic_v0`.

## Highest-Principle Boundary

The system remains a silicon-life substrate, not a request/response program.
This organ does not create chat, command handling, natural-language
understanding, terminal control, or a human-facing answer engine.

Allowed body-facing substance:

- byte bursts;
- fiber / lane numeric locators;
- carrier numeric locators;
- time windows;
- burst energy;
- interval rhythm;
- collision pressure;
- continuity score;
- prediction pressure;
- output burst pressure;
- return correlation;
- correction deviation;
- bounded trace history.

Forbidden body-side shortcuts:

- tokenizer, parser, vocabulary table, dictionary, grammar tree, or meaning
  database;
- token id, word id, text-understanding field, object name, concept id, label
  id, semantic readiness, symbolizer, naming gate, source identity, owner,
  ethics, or personality state;
- body-side `println!`, `eprintln!`, terminal prompt handling, shell execution,
  network command execution, or human-readable diagnostics;
- a module that writes answers on behalf of the body.

## High-Risk Term Boundary Notes

This spec necessarily uses high-risk external terms: `binary`, `character`,
`text`, `word`, `token`, `label`, `name`, `semantic`, `concept`, `identity`,
`owner`, `message`, `topic`, and `communication`.

1. External description vs internal ontology:
   these terms describe human-visible carriers, rejected shortcuts, or
   observer-side reports. They are not body ontology units.
2. Engineering index use:
   fiber ids, lane ids, carrier ids, burst refs, output refs, frame refs, and
   stream context ids are bounded engineering locators over signals.
3. Why not ontological:
   no body state records what bytes mean, who sent them, what a topic is, what
   a word is, or whether an output is a correct answer.
4. Why no highest-principle or language-axiom violation:
   the organ stores only numeric/time/curve traces and uses existing substrate
   return loops. Observer-side rendering may display bytes for research, but
   those renderings do not become body truth.

## Relationship To Existing Substrate

Existing modules remain lower-level channels:

```text
BinaryCharacterEar
  -> code_ingress
  -> evidence tension / active gap / prediction pressure

BinaryCharacterMouth
  -> framed_byte_egress
  -> action output / action consequence / returns
```

`code_ingress` is the spinal sensory channel for byte disturbance frames. It
does not own concurrent fiber separation, lane continuity, collision pressure,
or return correlation.

`framed_byte_egress` is the spinal motor channel for byte write-out frames. It
does not own lane selection, burst organization, output rhythm, or
return-driven mouth adaptation.

The new organs organize these channels without replacing them.

## BinaryCharacterEar

The ear receives raw binary character bursts from virtual or real providers.
Each burst is externally located by numeric carrier facts:

```text
BinaryCharacterBurstInput {
  fiber_id
  carrier_id
  t_start_us
  t_end_us
  bytes
  burst_energy
  framing_validity
}
```

`fiber_id` is not a person id. It is a physical or virtual input fiber locator.
`carrier_id` is not a topic id. It is a carrier surface locator.

The ear forms body-side lane traces:

```text
BinaryCharacterLaneTrace {
  lane_id
  fiber_id
  carrier_id
  last_burst_ref
  last_t_us
  byte_hash
  burst_count
  continuity_score
  recurrence_score
  collision_pressure
  prediction_pressure
  return_correlation
}
```

The ear must preserve ambiguity. If two fibers overlap or byte patterns collide
inside the same time window, the ear raises collision pressure instead of
forcing one lane to explain the other.

The ear writes each accepted burst into `code_ingress` using a stream context
derived from the active lane. This keeps all byte disturbance evidence visible
to the existing substrate.

## BinaryCharacterMouth

The mouth is a body-side binary character motor organ. It does not produce
answers. It produces byte bursts under substrate pressure.

Legal output facts:

```text
BinaryCharacterMouthState {
  active_lane_id
  output_pressure
  burst_length
  rhythm_score
  predicted_byte_hash
  last_output_ref
  output_count
  return_count
  return_match
  correction_deviation
}
```

The mouth starts from the ear's active lane pressure and existing active-gap /
evidence pressure. It organizes bounded bytes into `framed_byte_egress` frames.
The external world may render or route those bytes. The body itself does not
format a chat reply, execute a command, or choose a human recipient.

Return evidence comes back through:

```text
BinaryCharacterReturnInput {
  lane_id
  t_us
  returned_bytes
  magnitude
  correction_deviation
}
```

The mouth records return match and correction deviation, then forwards the
return through `observe_framed_byte_return` so action-consequence pairing
remains the single consequence substrate.

## Runtime Order

The body refresh cycle should include the new organs in this order:

```text
binary character ear ingress
-> code ingress evidence tension
-> active reasoning gap
-> prediction pressure field
-> binary character mouth drive
-> probe / action output
-> snapshot
```

The ear may be updated by provider threads before the global refresh cycle,
similar to other adapters. The mouth drive must happen after evidence pressure
is updated and before snapshot observation.

## Virtual Life Room Provider Boundary

The virtual life room provides virtual hardware only:

- input fibers from local text files, terminal-like test ports, network-like
  fixtures, or multiple simulated people;
- output devices that collect mouth byte bursts;
- return/correction providers that send byte returns back to the body.

The life room may know external labels such as "human A" or "network fixture"
for test setup. It must translate those into numeric fiber/carrier facts before
body ingress. It must not inject meaning, parse results, or choose answers.

## v0 Proof Targets

v0 must prove four primitive abilities:

1. Construction:
   multiple concurrent input fibers can be separated into stable lanes without
   treating fibers as identities.
2. Prediction:
   repeated byte bursts on the same lane increase continuity / recurrence and
   reduce local prediction pressure relative to first exposure.
3. Output organization:
   the mouth can produce bounded byte bursts through `framed_byte_egress`
   attached to the active lane.
4. Return-based primitive understanding:
   correction returns change mouth deviation / return match, and reentry on a
   trained lane is stronger than cold exposure.

These are not open natural-language chat claims. They are organ-level evidence
that binary character disturbance and binary character output are part of the
body loop.

## Acceptance Criteria

1. `BinaryCharacterEar` records concurrent bursts from at least three fibers
   as separate lanes.
2. overlapping bursts raise collision pressure without merging lanes.
3. repeated bursts on one lane raise recurrence and continuity.
4. each ear burst forwards a lane-scoped frame into `code_ingress`.
5. `BinaryCharacterMouth` emits bounded byte bursts through
   `framed_byte_egress` without human-readable body output.
6. return/correction input updates mouth deviation and forms
   `FramedByteReturn` action-consequence evidence.
7. snapshot roundtrip preserves ear and mouth observer-visible numeric state.
8. body runtime refresh order includes mouth drive after code-ingress evidence
   pressure and before snapshot.
9. virtual life room `--character-io` proves concurrent fibers, output bursts,
   returns, and primitive communication candidate evidence.
10. source hygiene rejects the forbidden ontology and request/response
    shortcuts listed above.

## Non-Goals

- No open natural-language chat.
- No tokenizer, parser, dictionary, grammar, or semantic engine.
- No user, speaker, topic, source identity, or ownership ontology in body.
- No command execution, shell bridge, network agent, or browser control.
- No replacement of PHOT, VIB, vocal, `code_ingress`, or `framed_byte_egress`.
- No claim that byte output is a correct answer.
