# Digital Disturbance Egress Substrate Design

Status: DRAFT v1
Date: 2026-05-06
Scope: `bionic_v0` framed-byte egress substrate

## 0. Purpose

This spec is the minimal egress slice of the broader 2026-05-06
computer-native code I/O substrate design. It narrows the first output
implementation to framed byte disturbance write-out while preserving the
broader ingress/egress contract.

The admissible first version is only this:

```text
body-side pressure
-> framed byte disturbance
-> action_output write epoch
-> ordinary return / numeric correction
-> temporal pairing
-> low-level projection update
```

The channel is a body-side output substrate. It is not a controller interface,
not a human exchange loop, and not a built-in interpreter of byte contents.

## 1. Highest-Principle Alignment

The system remains a silicon-life substrate, not a program organized around
external task exchange.

Therefore this substrate must satisfy:

- the body owns a continuously visible write state;
- every nonzero write enters shared substrate time;
- every write produces an `action_output` write epoch;
- returns enter through ordinary substrate observation paths;
- pairing uses time and numeric curve evidence;
- correction changes numeric projection traces only;
- observer rendering does not feed back as body truth;
- the body emits no human-readable diagnostics.

The substrate must not grow into a controller carrier, an execution router, a
human exchange loop, or a meaning authority.

## 2. High-Risk Boundary Notes

This document intentionally avoids installing discrete-language units in the
body. When it mentions byte frames, projections, traces, and correction, these
are engineering substrate facts only:

- `stream_id`, `frame_seq`, `write_seq`, and `window_id` are locators;
- byte payloads are quantized output disturbances;
- projection confidence is a numeric curve over observed support/deviation;
- correction deviation is a numeric pressure, not a judgment;
- observer rendering is research-side inspection only.

No field introduced by this spec may decide what a byte pattern means, who or
what produced it, what external task it belongs to, or whether an intended
world state has been reached.

## 3. Core State

The minimal body-side state is:

```text
FramedByteEgressState {
  phase
  active_stream_id
  current_write_seq
  next_frame_seq
  next_return_seq
  byte_cursor
  output_pressure
  projection_confidence
  correction_deviation
  correction_count
  last_frame_seq
  last_return_window_id
  last_update_t_us
  frame_traces
  return_traces
}
```

Frame traces contain:

```text
FramedByteFrameTrace {
  frame_seq
  stream_id
  write_seq
  start_t_us
  end_t_us
  byte_len
  byte_checksum
  pressure
}
```

Return traces contain:

```text
FramedByteReturnTrace {
  return_seq
  stream_id
  window_id
  return_t_us
  byte_len
  byte_checksum
  magnitude
  correction_deviation
}
```

These structures are byte-shape and time traces. They carry no external
purpose field.

## 4. Minimal Write Flow

`begin_framed_byte_egress(now, stream_id, bytes, pressure)` must:

1. clamp `pressure` into `[0,1]`;
2. apply `ActionOutputFeedback::Begin` on `OutputChannelKind::FramedByteEgress`;
3. read the resulting `write_seq`;
4. append one frame trace;
5. set phase to active;
6. expose the write through snapshot and observer traces.

`adjust_framed_byte_egress(now, bytes, pressure)` must:

1. reuse the active `write_seq`;
2. apply `ActionOutputFeedback::Adjust`;
3. append another frame trace;
4. update pressure, cursor, frame count, and time.

`stop_framed_byte_egress(now)` must:

1. apply `ActionOutputFeedback::Stop`;
2. set the egress phase to idle;
3. leave historical traces observable.

No function in this flow may treat byte contents as an external task directive.

## 5. Minimal Return And Correction Flow

`observe_framed_byte_return(now, stream_id, bytes, magnitude, deviation)` must:

1. create an ordinary `action_consequence` window with
   `ConsequenceModalityKind::FramedByteReturn`;
2. attempt normal temporal pairing with the latest legal write epoch;
3. append a return trace;
4. update `correction_deviation`;
5. nudge `projection_confidence` numerically.

Deviation near `1.0` weakens projection confidence. Deviation near `0.0`
strengthens projection confidence. This is not a meaning judgment; it is only a
low-level support/deviation curve.

Unpaired returns remain valid observations.

## 6. Snapshot And Observer

Snapshot may expose:

- phase;
- active stream locator;
- write sequence;
- frame and return counts;
- last frame sequence;
- last return window id;
- pressure;
- projection confidence;
- correction deviation;
- correction count;
- last update time.

Observer may render these values compactly, for example:

```text
fbe:ph1|sid7|ws1|nf1|nr1|lf1|rw1|p0.400|pc0.560|dev0.200
```

That row is a research rendering of substrate traces. It must not feed back as
body truth.

## 7. Admission Checklist

The minimal implementation is admissible only if:

1. framed bytes produce an `action_output` write epoch;
2. adjustment reuses the active write epoch;
3. stop idles the output state and epoch phase;
4. returns enter as ordinary consequence windows;
5. write/return pairs form only by substrate time evidence;
6. correction changes numeric projection traces only;
7. snapshot and observer expose only numeric/time/byte-shape traces;
8. body code has no human-readable diagnostics;
9. no controller carrier or external task exchange path is introduced.

## 8. Non-Goals

This spec does not implement active reasoning, external task operation,
interpretation of byte contents, remote operation, document parsing, learned
language binding, or long-horizon communication behavior.

Those require later specs and must still preserve the same body/observer
boundary.
