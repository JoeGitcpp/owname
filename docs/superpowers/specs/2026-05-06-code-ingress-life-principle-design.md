# Code Ingress Life-Principle Design

Status: DRAFT CANDIDATE
Date: 2026-05-06
Scope: `bionic_v0` minimal `CodeIngressAdapter`

## 0. Purpose

This spec strictly redesigns `CodeIngressAdapter` as a body-side sensory organ
for external computer-native byte disturbances.

The first implementation target is deliberately narrow:

`dedicated framed byte carrier -> viewport code-ingress trace -> snapshot`

It does not implement code understanding, command execution, shell access,
terminal control, language learning, or code egress. It only proves that
framed byte disturbances can enter body time as substrate traces and remain
observer-visible without becoming human text inside the body.

## 1. Highest-Principle Alignment

The highest principle remains:

- the system is a silicon-life substrate, not request/response software;
- body-side state must remain numeric, temporal, curve-like, bounded, and
  reversible;
- observer-side rendering must stay outside the body;
- language, naming, tokens, labels, commands, and source identity must not
  become internal ontology.

Therefore `CodeIngressAdapter` must be treated like a sensory organ, not a
human input interface.

Legal shape:

```text
external byte disturbance
-> framed carrier sample
-> timed numeric trace
-> viewport
-> snapshot
-> observer rendering
```

Illegal shape:

```text
stdin / terminal / socket
-> read line
-> parse text
-> understand command
-> execute or respond
```

## 2. High-Risk Term Boundary Notes

This spec necessarily uses high-risk words: `code`, `text`, `string`, `token`,
`label`, `name`, `word`, `semantic`, `concept`, `identity`, `owner`,
`terminal`, `shell`, and `command`.

They are bounded as follows:

1. External description vs internal ontology:
   these words describe external artifacts, rejected routes, observer-side
   renderings, or engineering carriers. They are not body ontology units.
2. Engineering index use:
   stream ids, frame ids, projection refs, sequence window ids, and evidence
   refs are engineering locators only.
3. Why not ontological:
   no body-side field may assert what bytes mean, what a byte sequence is
   called, who owns it, what source identity it has, or what command it
   represents.
4. Why no highest-principle or language-axiom violation:
   the body records disturbance timing, bounded byte samples, numeric
   projection confidence, recurrence, ambiguity, and error. Any human-readable
   rendering remains observer-side and must not flow back as body truth.

If a future implementation cannot maintain these four answers, it is
inadmissible.

## 3. Core Definition

`CodeIngressAdapter` is a dedicated sensory adapter for computer-native byte
disturbances.

`Code` is an engineering name for the carrier family. It does not mean the body
has an internal concept of code, program, shell, terminal, command, word, or
meaning.

`Frame` is an engineering boundary. It can be a length-prefixed carrier record,
datagram, pipe packet, or ring-buffer entry. It is not a word boundary, command
boundary, sentence boundary, token span, AST node, or meaning unit.

`Byte disturbance` is the only first-version input substance.

## 4. Minimal Body-Side State

The first version introduces one independent viewport subtree:

```text
CodeIngressState {
  stream_count
  recent_frames
  projection_candidates
  sequence_windows
  last_update_t_us
}
```

### 4.1 Frame Trace

Allowed frame facts:

```text
CodeIngressFrameTrace {
  frame_seq
  stream_context_id
  t_start_us
  t_end_us
  byte_len
  byte_hash
  byte_transition_energy
  byte_distribution_skew
  framing_validity
  evidence_ref
  byte_sample
  byte_sample_len
}
```

`byte_sample` must be bounded by a small fixed cap. It is equivalent to an
audio sample or camera sample: externally readable, but not the body's thinking
unit.

Forbidden frame facts:

- text meaning;
- token id;
- word id;
- concept id;
- object name;
- command meaning;
- shell state meaning;
- terminal prompt meaning;
- source identity;
- owner;
- success or failure semantics.

### 4.2 Projection Candidate

A projection candidate is a numeric candidate view over the same bytes:

```text
CodeProjectionCandidate {
  projection_ref
  frame_seq
  decoder_kind_code
  confidence
  validity
  error_magnitude
  code_unit_count
  last_update_t_us
}
```

`decoder_kind_code` is an engineering code. Observer tooling may display names
for common decoders, but the body stores only numeric codes and confidence
curves.

Multiple projection candidates may coexist for the same frame. The first
version must preserve ambiguity instead of collapsing bytes into one privileged
text interpretation.

### 4.3 Sequence Window

A sequence window records local continuity and recurrence:

```text
CodeSequenceWindowTrace {
  window_id
  stream_context_id
  start_frame_seq
  end_frame_seq
  t_start_us
  t_end_us
  recurrence_score
  boundary_confidence
  ambiguity_level
}
```

This is not a token span, line, sentence, command, or AST range. It is only a
time-local continuity trace.

## 5. Adapter Contract

Each tick, the adapter may:

1. non-blockingly drain the dedicated framed carrier;
2. stamp frame timing with shared body time;
3. assign monotonic `frame_seq` and `evidence_ref` handles;
4. compute bounded numeric byte features;
5. update projection candidates;
6. update sequence windows;
7. write only the `code_ingress` viewport subtree.

Each tick, the adapter must not:

- block waiting for human input;
- parse text for meaning;
- call an LLM;
- call OCR;
- tokenize;
- execute commands;
- open a real shell, terminal, SSH session, or write-capable remote carrier;
- write object slots;
- write language binding events in the first version;
- write action-output state;
- print body diagnostics.

Quiet, stale, and carrier-fault conditions must be represented by numeric
health/validity state, not by body-side text output.

## 6. Viewport, Snapshot, Observer

Viewport owns the body-side truth:

- `Viewport` stores `CodeIngressState` behind a lock;
- adapter writes only through focused viewport APIs;
- other systems may read the state but must not reinterpret bytes as meaning.

Snapshot exposes trace truth:

- stream count;
- recent frame metadata;
- bounded byte samples;
- projection candidate summaries;
- sequence-window summaries;
- numeric health and ambiguity state.

Observer may render bytes as hex or human-readable text for research use. That
rendering is not body state. It must not feed back into the body unless a later
explicit ingress or correction channel is designed and admitted.

## 7. Relationship To Existing Substrate

This spec narrows the ingress side of
`2026-05-06-computer-native-code-io-substrate-design.md`.

It intentionally does not implement:

- `CodeEgressOrgan`;
- shell or terminal carrier admission;
- automatic language binding;
- glyph/PHOT alignment;
- command/result feedback;
- code understanding.

Those may only be admitted after this minimal sensory substrate is proven by
viewport and snapshot traces.

`CodeIngressAdapter` also does not masquerade as VIB, PHOT, or MECH. Byte
disturbances are computer-native ingress traces, not sound, light, or touch
events.

## 8. First-Version Acceptance Criteria

The first implementation is admissible only if all are true:

1. dedicated framed bytes enter `viewport.code_ingress`;
2. frame traces carry body time, numeric features, bounded byte samples, and
   evidence refs;
3. multiple projection candidates can coexist for one frame;
4. sequence windows track continuity without becoming token spans;
5. snapshot roundtrips frame, projection, and sequence-window traces;
6. observer renders snapshot truth only;
7. body source contains no `println!` or `eprintln!` diagnostics in the new
   ingress path;
8. no `semantic_readiness`, `symbolizer`, `concept_id`, token ontology, naming
   gate, source identity, owner field, shell meaning, or command meaning is
   introduced;
9. first-version code ingress does not write object slots, language binding,
   action output, or action consequence state.

## 9. Closing Position

The minimal `CodeIngressAdapter` is not a way for a user to talk to the body.
It is a way for the body to sense a computer-native world disturbance.

Only after that disturbance can enter shared time, remain ambiguous, produce
projection/error history, and be observed through snapshots should later
binding, egress, glyph alignment, or feedback loops be considered.
