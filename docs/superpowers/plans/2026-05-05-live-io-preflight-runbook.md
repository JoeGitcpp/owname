# Live I/O Preflight Runbook

This runbook is observer-side. It must not become a body module, a startup
diagnostic gate, or a source of body-side human-readable output.

## Deterministic Preflight

Run from `/Users/joe/unnamed/.worktrees/live-io-preflight-life-sign`:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight
```

Expected evidence:

- vocal output start creates exactly one write epoch;
- the vocal imitation loop creates an epoch when it starts output;
- active probe-law output changes the vocal carrier used by the audio path;
- surprise, relation-instability, and attention-adjust write triggers use the
  same physical carrier path as their write epochs;
- later ingress can pair with a real write epoch;
- ingress without a write epoch stays unpaired;
- overlapping windows stay separate numeric windows;
- synthetic PHOT structure reaches slots, spatial state, and relations;
- snapshot exposes action output, consequence pair, vocal organ state, PHOT
  slots, and relation state.

## Related Guards

Run focused guards:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_runtime
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop
cargo test --manifest-path bionic_v0/Cargo.toml --test real_phot_adapter
cargo test --manifest-path bionic_v0/Cargo.toml --test body_no_human_output
```

Expected evidence:

- the vocal adapter still uses the real audio output path and avoids stdout /
  stderr diagnostics;
- action-consequence pairing still rejects phantom writes;
- PHOT adapter behavior still writes real substrate slots and timing fields;
- `body` still contains no human-readable print output.

## Live Hardware Smoke

Use this only after deterministic tests pass. Run the body and observer as
separate processes:

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin body -- runtime
```

```bash
cargo run --manifest-path bionic_v0/Cargo.toml --bin observer -- runtime 20
```

Observer-side signs to inspect:

- `out:` write epoch count increases when body output is active;
- `acq:` windows appear when microphone or camera ingress changes;
- `acq:` pairs appear only when a real write epoch precedes a plausible
  ingress window;
- PHOT health is visible as OK, QUIET, STALE, or FAULT instead of silent;
- PHOT slots carry spatial state when visible motion is present;
- relation counts increase when multiple active slots are present;
- vocal numeric fields change during vocal activity.

Stop the live smoke if:

- `body` emits human-readable diagnostics;
- output activity does not create a write epoch;
- ingress changes create pairs without any earlier write epoch;
- camera or microphone failure is silent instead of visible in snapshot /
  observer state.

## Admission Boundary

Passing this runbook admits the substrate to active probe-feedback experiments.
It does not prove communication, target success, recognition, source
attribution, or object meaning. Those remain outside this preflight contract.
