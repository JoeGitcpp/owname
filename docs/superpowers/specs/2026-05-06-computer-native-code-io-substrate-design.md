# Computer-Native Code I/O Substrate Design

Status: DRAFT v0
Date: 2026-05-06
Scope: `bionic_v0` computer-native code ingress / egress substrate

## 0. Purpose

This spec defines a silicon-native input/output substrate for framed byte
streams.

The goal is to add two body channels:

- `CodeIngressAdapter`: a body-side sensory channel for computer-native byte
  disturbances.
- `CodeEgressOrgan`: a body-side output organ for writing computer-native byte
  disturbances.

These channels let the body couple directly to computer systems without making
text, strings, words, tokens, terminals, shells, or SSH into its thinking
language. They are substrate I/O channels, not a chat layer and not a tool
router.

The first version uses a dedicated framed byte stream. The stream may later be
backed by a pipe, socket, PTY, local terminal, remote terminal, or SSH transport,
but those carriers are outside the body ontology. The body only sees timed code
disturbances, write traces, projections, prediction errors, and correction
history.

## 1. Highest-Principle Alignment

The highest principle remains unchanged:

- the body is a silicon-life substrate, not request/response software;
- body code must not print human-readable diagnostics;
- observation belongs to observer-side tools;
- language binding must not become object ontology;
- output must be a body write event with time structure and consequence return.

This spec therefore forbids implementing code I/O as:

- `println!` / `eprintln!` from the body;
- a terminal chatbot loop;
- a tokenizer-driven reasoning path;
- a dictionary or word table;
- a command executor that treats shell/SSH as built-in meanings;
- an observer bridge that translates human text into body truth;
- a source of internal concept ids, labels, names, or semantic readiness.

The body may use operating-system I/O primitives because every physical output
organ also needs a carrier. This is analogous to vocal output using the audio
device. The carrier is admissible only when body-side state remains numeric,
temporal, reversible, and observable through snapshots without human-readable
body output.

## 2. High-Risk Term Boundary Notes

This document necessarily uses words such as `text`, `character`, `string`,
`word`, `code`, `token`, `label`, `name`, `semantic`, `concept`, `terminal`,
`shell`, and `SSH`. They are bounded as follows.

1. External description vs internal ontology:
   these words describe external computer artifacts, rejected architecture
   routes, or engineering carriers. They are not body ontology units.
2. Engineering index use:
   stream ids, frame ids, decoder kind codes, sequence windows, projection
   handles, and evidence refs are engineering locators only.
3. Why not ontological:
   no field may assert what a string "means", what an object "is called", or
   whether a code sequence is a real concept. Meaning can only form later as a
   reversible binding to objects, actions, relations, time structures, and
   feedback.
4. Why no highest-principle or language-axiom violation:
   the body processes raw disturbance curves and write traces. Decoder outputs
   are projections over the same disturbance, not a token mother tongue.

Any future code or test using these terms as internal truth fields is
inadmissible.

## 3. Core Definition

### 3.1 Code Disturbance Element

A `code disturbance element` is a time-bounded external computer disturbance.
Its lowest representation is raw code units, usually bytes.

It may have multiple projections:

- raw bytes;
- candidate decoded code units;
- terminal/control-sequence projection;
- glyph or visual-shape projection;
- sequence-window position;
- binding candidates;
- correction history.

None of these projections is meaning by itself.

The same raw bytes may support multiple projections at once. The body must
preserve ambiguity until later evidence, feedback, or context changes the
projection strengths.

### 3.2 Native But Replaceable

Code I/O is native to a silicon body in the same sense that vocal output is
native to the current body once the vocal organ exists. Native does not mean
fixed meaning. It means:

- the body can continuously receive and write computer disturbances;
- these disturbances enter shared time;
- writes produce write epochs;
- ingress and egress can be paired by delay and curve evidence;
- learning can revise how projections and bindings behave.

The carrier is replaceable. A dedicated pipe, socket, PTY, SSH-backed stream, or
other byte carrier can all satisfy the same body contract if they expose the
same framed disturbance interface.

## 4. Frame Contract

The first version uses framed byte streams. A frame is an engineering boundary,
not a word boundary and not a meaning boundary.

### 4.1 Ingress Frame

An ingress frame contains only substrate facts:

```text
CodeIngressFrame {
  frame_seq
  t_start_us
  t_end_us
  stream_context_id
  raw_bytes
  decoder_projection_refs
  sequence_window_id
  evidence_ref
}
```

Allowed properties:

- monotonic sequence handles;
- shared body time;
- byte payload;
- stream context handle;
- references to decoder projections;
- references to sequence windows and evidence.

Forbidden properties:

- text meaning;
- token id;
- word id;
- concept id;
- object name;
- command meaning;
- source identity truth;
- shell/SSH semantic state.

### 4.2 Decoder Projection

A decoder projection is a candidate view over the same raw disturbance:

```text
DecoderProjection {
  projection_ref
  decoder_kind_code
  confidence
  code_units
  validity
  glyph_projection_ref
  last_update_t_us
}
```

`decoder_kind_code` is numeric and engineering-only. Examples may include UTF-8,
UTF-16, ASCII, terminal control, or binary-fragment projections, but these names
are observer-side descriptions. The body stores numeric projection codes and
confidence curves.

Projection confidence changes through prediction error and correction feedback.
No projection may become a semantic authority.

### 4.3 Sequence Window

A sequence window tracks local continuity:

```text
CodeSequenceWindow {
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

Sequence windows help detect repetition, boundary stability, and local ordering.
They are not token spans or sentences.

## 5. Code Ingress Adapter

`CodeIngressAdapter` reads a dedicated byte-stream carrier and converts it into
timed frames.

Its responsibilities are:

1. collect raw bytes from the carrier;
2. assign shared body time;
3. create frame boundaries using stream-local heuristics and backpressure;
4. maintain decoder projection candidates;
5. maintain sequence windows;
6. publish substrate traces to the viewport/snapshot path;
7. emit binding-event candidates when repeated patterns co-occur with active
   object, action, relation, or time structures.

It must not:

- parse text for meaning;
- call an LLM;
- call OCR;
- allocate tokens;
- allocate concept ids;
- execute commands;
- treat a terminal prompt as a built-in object class;
- print body diagnostics.

### 5.1 Input Learning Flow

```text
dedicated byte stream
-> raw bytes
-> ingress frames
-> competing decoder projections
-> sequence windows
-> binding-event candidates
-> feedback / prediction error
-> projection and binding updates
```

The first learning target is stable channel coupling:

- frame boundaries become more stable;
- decoder projections compete and revise;
- repeated byte patterns become easier to recall;
- glyph projection links can form when visual evidence exists;
- binding candidates remain reversible.

The first learning target is not language understanding.

## 6. Code Egress Organ

`CodeEgressOrgan` writes code disturbances. It is an output organ, not a string
generator.

Its state is numeric and temporal:

```text
CodeEgressOrganState {
  phase
  active_stream_id
  write_seq
  target_projection_ref
  output_pressure
  byte_cursor
  pending_bytes
  emitted_bytes_ref
  last_update_t_us
}
```

`pending_bytes` and emitted byte refs are output curves in a quantized carrier.
They are comparable to audio samples in the vocal path: externally readable,
but not the body's thinking unit.

### 6.1 Output Flow

```text
object / action / relation / time activity
-> binding activation
-> candidate code projection
-> output pressure
-> framed byte write
-> write epoch
-> ingress return / correction feedback
-> prediction error
-> projection, binding, and egress update
```

The organ writes through the action-output substrate. Every nonzero write must
have a write epoch and be eligible for consequence pairing.

### 6.2 Output Prediction And Correction

The egress organ predicts low-level consequences:

- whether the carrier accepts bytes;
- whether a return window appears;
- whether the return timing matches history;
- whether returned bytes or visual glyphs match expected projections;
- whether correction feedback supports or weakens the projection.

It does not predict truth or meaning directly. Meaning-level correction belongs
to the language binding layer through binding events.

## 7. Stream Contexts And Carriers

A `stream_context_id` is an engineering handle for a byte carrier and its local
history.

Allowed first-version carrier:

- dedicated framed pipe/socket/fd created for the body.

Admissible later carriers:

- PTY-backed local terminal;
- file/buffer stream;
- local socket;
- SSH-backed byte stream;
- remote terminal stream.

The body must not contain a built-in concept that says "this is SSH" or "this
is a shell". Those are learned context regularities or observer descriptions.
The substrate only records stream handles, frames, writes, returns, and
prediction errors.

## 8. Relationship To Language Binding

Code I/O feeds the existing language binding contract. It does not replace it.

Code ingress may generate binding-event candidates when a code disturbance
co-occurs with active internal structures:

```text
code disturbance element
+ active object/action/relation/time structure
+ later support/deviation feedback
-> BindingEvent
```

The binding layer may later strengthen, weaken, or retract those candidates.
Binding traces remain reversible and evidence-backed.

No code disturbance may promote an object into existence, assign a real name, or
create an internal concept entity.

## 9. Relationship To Visual Glyph Learning

Some code disturbances have visual projections when rendered by an external
surface. These projections can align with PHOT/glyph evidence:

```text
raw bytes / code-unit projection
<-> rendered glyph shape
<-> PHOT region / surface projection
<-> binding evidence
```

This enables the body to learn both:

- how a disturbance is encoded in a computer stream;
- how that disturbance appears visually.

The visual projection is not privileged over the byte projection. Both are
external expressions of a disturbance family.

## 10. Prediction And Error Boundaries

Low-level prediction covers:

- frame boundary likelihood;
- decoder projection confidence;
- sequence recurrence;
- output acceptance;
- return timing;
- byte/glyph projection alignment;
- correction support or deviation.

Low-level prediction does not cover:

- statement truth;
- command intention;
- user identity;
- source trustworthiness;
- word meaning as ontology.

Higher-level meaning, trust, and communication habits must form through
binding, history, feedback, and active verification.

## 11. Safety And Admission

The first version must use a dedicated sandbox stream. It must not default to a
real SSH session, real shell, or write-capable remote machine.

Admission to stronger carriers requires observer-visible evidence that:

- writes enter action-output epochs;
- ingress returns are recorded through ordinary code ingress;
- ambiguous returns remain ambiguous;
- correction feedback can weaken bad projections;
- no body stdout/stderr diagnostics exist;
- no tokenizer or semantic gate has been introduced.

The first version may prove the substrate with synthetic and local carriers
before any remote carrier is considered.

## 12. Snapshot And Observer

Snapshot may expose numeric and byte-level traces for observer inspection:

- stream count;
- frame count;
- recent frame metadata;
- projection confidences;
- sequence-window counts;
- egress phase;
- write epoch handles;
- prediction error summaries.

Observer may render bytes as human-readable text for research inspection. That
rendering remains outside the body. It must not feed back as body truth unless
it re-enters through an explicit correction or ingress channel.

## 13. Non-Goals

This spec does not implement:

- a text chat interface;
- a tokenizer;
- a parser as ontology;
- a shell controller;
- an SSH agent;
- a source identity module;
- a truth evaluator;
- an internal word dictionary.

Those may appear later only as external carriers, observer tools, or learned
regularities, never as body ontology.

## 14. First-Version Acceptance Criteria

A first implementation satisfies this spec only if:

1. code ingress can read a dedicated framed byte stream into numeric timed
   frames;
2. multiple decoder projections can coexist for the same raw bytes;
3. code egress can write framed bytes through action-output write epochs;
4. ingress return or correction feedback can adjust projection confidence;
5. snapshot exposes trace state without body text output;
6. tests guard against stdout/stderr diagnostics, token ontology, concept ids,
   and semantic readiness reintroduction;
7. no shell, terminal, SSH, word, or string meaning is built into body logic.

This is the silicon-native I/O substrate. Language and computer fluency must
grow from binding, prediction, correction, and history on top of it.
