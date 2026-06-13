# aborn / bionic_v0 缺陷审计报告

> 2026-06-13 · 多代理审计(57 agents)+ 逐条对抗式核验 · 范围:全项目 + aborn.live 通信
> 生成: full-project-defect-audit workflow

I have everything I need from the verified findings. Writing the report now.

---

# Defect Report — aborn / bionic_v0
**Scope:** whole project + aborn.live live comms. 34 confirmed defects, each adversarially verified. The verification notes corrected several severities down from how they were first framed; the groupings below use the *verified* severity, not the original claim.

---

## CRITICAL
None. No defect causes guaranteed data destruction, remote code execution, or unconditional crash for all users. (Several were initially framed as critical and corrected down on verification — that correction is itself a useful signal: the failures are quiet, not loud.)

---

## HIGH (7)

**1. Mainline feeder clobbers the merged master every 120s — visitor learning is permanently destroyed**
What/where: `gateway/nursery_feeder.mjs:395-416` `copyFileSync` overwrites the exact `master_vocal_target_recall.bin` / `master_vocal_memory.bin` / `master_phoneme_memory.bin` that `merge_artifacts.rs:43,63,83` merges into; merge runs hourly, feeder harvest every 120s, and the feeder seeds from master only **once** at startup (`nursery_feeder.mjs:464`).
Why it matters: last-writer-wins, feeder dominates ~30:1. Every visitor branch's vocal/phoneme contribution merged at minute 0 is gone within ≤2 min. The entire branch→pool→mainline distributed-training pipeline is a **no-op in steady state**. This is a primary cause of "skill memories are static."
Fix: make `merge_artifacts` the single writer of `master_*.bin`; have the feeder harvest into the pool as `{ts}_mainline__<name>.bin`, or re-seed from merged master before each harvest. Never `copyFileSync` over the merged files.

**2. Auth layer is fully disabled — `requireAuth()` hardcoded `return false`**
What/where: `gateway/auth.mjs:234-240`; gate at `server.mjs:535` can never fire.
Why it matters: every visitor is admitted as anon and gets a real OS body process spawned. The entire OTP/email/phone system, per-email rate limits, and signed cookies are dead code in the admission path. The only barrier is upstream Cloudflare Access, which the code cannot verify and which the project's own notes say mangles tunnel frames.
Fix: gate resource-bearing paths — verify a CF Access JWT in-process against the team JWKS, or flip `requireAuth` to require a valid session cookie once the client OTP UI exists.

**3. `developmental_maturity` is EMA-merged across branches like a skill axis — phantom maturity propagates**
What/where: `gateway/training_pool.mjs:137-145,207-215` blindly EMA-merges every f64 from offset 28 to EOF, including the trailing maturity/efficacy/suppression/familiarity scalars; harvested/seeded whole via `nursery_feeder.mjs:425-441`.
Why it matters: maturity is designed as a monotonic developmental *clock* (the single chokepoint, `viewport.rs:2302-2313`), not a skill measure. Merging it makes master maturity ratchet toward the highest-maturity branch regardless of competence. This is the mechanism behind "maturity pinned at 1.0 while efficacy stuck ~0.24." Unlike efficacy (which has a pool-admission gate), maturity has no guard.
Fix: exclude the trailing scalars from the byte-range merge; recompute maturity from skill-artifact evidence after merge. Maturity should be *derived*, not transported.

**4. Body sandbox silently disabled when bwrap is absent — no warning, no refusal**
What/where: `gateway/server.mjs:116-119` falls back to running the body unconfined whenever `fileExists(BWRAP)` is false; `scripts/server_setup.sh` never installs bubblewrap, so a fresh server is exactly this case.
Why it matters: bodies process attacker-controlled visitor frames. With the jail gone, a body compromise reaches SSH keys, cloudflared tunnel creds, source tree, and other sessions — the exact assets the code comment says are walled off — and the operator gets zero signal. Fail-open security control matching the out-of-box state.
Fix: fail closed (refuse to spawn + loud log) when SANDBOX is intended but bwrap is missing, unless `ABORN_SANDBOX=0` is explicit. Add `apt-get install -y bubblewrap` to `server_setup.sh`; log the chosen mode at startup.

**5. Stateless 7-day session cookies: non-revocable + forgeable on secret leak**
What/where: `gateway/session_secret.mjs:13-32`, `auth.mjs:101-110`, `phone_auth.mjs:78-86`; verify path consults no server-side store (`auth.mjs:112-129`).
Why it matters: a stolen cookie is valid 7 days; `/auth/logout` only clears the requesting client. The signing secret falls back to an on-disk `../.session_secret` because no env/unit sets `ABORN_SESSION_SECRET`. If that file leaks (e.g. via the unconfined-body path in #4, or a backup), an attacker forges a cookie for any email/phone and impersonates any user's instance and memory. Same key signs both cookie types.
Fix: set `ABORN_SESSION_SECRET` in the unit; add a server-side revocation list or per-user token version in the payload; bind cookie type into the HMAC input.

**6. 1006 / abrupt-drop reconnect is not handled — the precise failure the server keepalive was built to survive dead-ends the user**
What/where: `site/src/views/Commune.tsx:48-55,205-208`; server keepalive at `gateway/server.mjs:511-527`.
Why it matters: the server deliberately pings every 20s and does NOT terminate on missed pong because CF strips control frames — it expects 1006 drops. But the client maps every unknown close code (incl. 1006) to `'kicked'`, which shows a modal with **no retry button** (`Commune.tsx:481`), while the copy literally tells the user "You can retry." No reconnect/backoff anywhere. Routine mobile lock-screen / NAT idle on the primary platform → dead-end screen.
Fix: treat 1006/1005/unknown as transient and auto-reconnect with backoff (re-run hello/XOR handshake); at minimum always show Retry for `'kicked'`.

**7. Governor/registry desync on resume-from-queue lets running instances exceed `INSTANCE_MAX`**
What/where: `gateway/server.mjs:402-408` (DELETE resume path); `gateway/resource_governor.mjs:44-54`.
Why it matters: `onDestroy` pops the next suspended id and returns it, but the caller sets it `'running'` in the registry without ever calling `resourceGovernor.onStart()` or re-adding to `#running`. The resumed instance is invisible to the governor, so `canStart()` undercounts and subsequent POSTs admit more bodies than the cap. The orphaned id can also never free its slot.
Fix: re-add the resumed id to `#running` in `onDestroy` (or call `onStart` on the resume branch). Keep registry state and governor set in lockstep.

**8. Supabase Postgres (5432/6543) + Kong (8000/8443) reachable off-box; `harden_db_ports.sh` un-run**
What/where: `scripts/harden_db_ports.sh:4-7,33-36` — the script's own header documents LAN-verified exposure of raw Postgres; nothing in `deploy_aborn.sh`/`server_setup.sh` invokes it and no iptables persistence exists.
Why it matters: confirmed LAN-reachable raw Postgres = direct DB access bypassing app auth, lateral movement on a shared home network. (Public reachability is unverified — gated on router port-forwarding — which is why this is high, not critical.)
Fix: run `harden_db_ports.sh --all` + `netfilter-persistent save`; better, bind Supabase/Docker to `127.0.0.1` only; add the step to `server_setup.sh` so a rebuild can't regress it.

**9. Unauthenticated IP-keyed endpoints allow trivial global-capacity DoS**
What/where: `gateway/server.mjs:442-453,556-578`; `security.mjs:102-111`.
Why it matters: auth disabled (#2) + `GLOBAL_MAX=8` means ~4 honest IPs (2 each) pin every session slot and spawn up to 8 bodies, denying service to everyone on top of the always-running feeder. (Header-spoof angle is overstated for the CF-tunnel topology, but the honest-IP exhaustion is real and self-heals only after the attack stops.)
Fix: require auth before spawning a body; add a global concurrent-spawn budget independent of IP and a per-IP cooldown after exit.

**10. B1 cross-layer "relation rules that must survive death" are serialized but never restored**
What/where: restore at `bionic_v0/src/bin/body.rs:384-396` reads **only** `snap.axis_xlayers`; `slot_xlayers` (commented "死后保留" at `snapshot.rs:225-229`) has no restore path and no setter exists (`grep` for `restore_slot_*` returns nothing).
Why it matters: per-slot B1 relation rules are reborn from zero every restart while the cheap scalar maturity persists — directly contradicts the in-code durability promise and feeds the "skill static / maturity decoupled" symptom. (Slot *towers* are intentional telemetry and axis-level B1 does persist, so impact is narrower than full amnesia — this is why it lands high, not critical-wide.)
Fix: add `Viewport::restore_slot_cross_layer` and call it at boot, or move durable B1 state into `learned_bias.bin` which *is* restored.

**11. Live body NEVER forms language bindings — only decays them**
What/where: `bionic_v0/src/bin/body.rs:436-440` ticks only maintenance/resonance (decay/upkeep); every formation entry point (`apply_language_binding_event`, `bind_latest_mark_to_target`, `try_form_binding_event_from_pair`) is called **only** from `curriculum.rs`, the offline harness the deployed body never runs.
Why it matters: in production a binding can only decay, never be created or reinforced. Symbol grounding is structurally dead — a prime "why language never emerges" root cause hiding behind code that compiles and looks wired.
Fix: wire a live formation path from the body's auditory-visual co-occurrence tick into `apply_language_binding_event`; add an integration test that runs the *body* tick loop and asserts a binding forms.

---

## MEDIUM (16)

**Comms / tunnel**
- **Uplink senses frame is 230 KB raw float32, never downscaled/compressed** — `Commune.tsx:41,57-92,216-223`; `security.mjs:19,30`. The black-screen fix only shrank the *downlink*; the uplink overruns the ~100 KB/s CF tunnel, causing stale/dropped vision and plausible startup black-screen on slow links. The body averages the grid into 4×4 regions anyway, so the float precision is wasted. Fix: send 40×30 uint8 RGB + gzip (~3.6 KB vs 230 KB); update `FRAME_BYTES`/`parse_frame`/`validateFrame` on both ends.
- **Session-quality harvest can copy half-written checkpoints** — `server.mjs:824-885`. Vocal artifacts use atomic tmp+rename; the personal-file copy (`:831-837`) and `learned_bias` `collect_to_pool` (`:859-867`) do not — they trust a fixed 3s SIGTERM grace. A slow flush corrupts persistent memory / poisons the pool. Fix: wait for `child.on('exit')` and apply tmp+rename everywhere.
- **Visitor harvest Gate 2 needs 20 object recognitions** — `training_pool.mjs:84-105`; degraded-vision tunnel users barely accumulate recognitions, so the gate rarely passes for fresh sessions and the pipeline collects near-zero visitor data. Compounds #1. (Softened by seeded persistent recognition counts.) Fix: add a vocal-only gate path; surface a pooled-vs-merged counter.
- **Config drift: production unit sets no `ABORN_MAINLINE` etc.** — `scripts/server_setup.sh:47-74`. A fresh `server_setup.sh` run silently disables the mainline feeder; the live 22h run came from a hand-edited unit not in the repo. Fix: add `ABORN_MAINLINE=1` to the committed unit.
- **`merge_pool` noise-gate reuses the admission threshold + `alpha=0.05` vs unbounded pool** — `training_pool.mjs:28-30,207-217`. Master barely moves; good sessions diluted; `score` is computed then discarded so a rich and a marginal session nudge equally. Fix: decouple the merge gate from admission, weight alpha by `score`, log merged/skipped.

**Auth**
- **OTP brute-force / abuse window** — `auth.mjs:185-202`, `identity.mjs:66-84`, `server.mjs:305-317`. No per-IP rate limit on any `/auth/*` or `/admin/*` route (the limiter only guards WS upgrades). Enables OTP/SMS bombing and unfriendly endpoint hammering. (The per-OTP 3-attempt cap genuinely bounds single-account ATO — the original "unbounded guessing" framing was wrong.) Fix: per-IP limiter at the top of the HTTP handler.
- **Admin secret compared with non-constant-time `===`, no throttle** — `server.mjs:416-420`. The codebase already uses `timingSafeEqual` for cookies. Endpoint suspends instances / changes global capacity → DoS lever. Fix: length-guarded `timingSafeEqual` + rate limit + allowlist.
- **MemoryStore + `instances.json` write raw email/phone to disk** — `memory_store.mjs:4-9`, `server.mjs:151,249-254`. Directory names like `email%3Auser%40example.com` contradict the documented "SHA-256, no raw email on disk" design (which *is* honored for the binary checkpoint dir — proving the team knows the pattern). Fix: hash the identity key before use as dir/registry key.

**Vocal learning loop**
- **vocal_memory prototypes born then erased — pool nets to zero** — `vocal_memory.rs:33-41,80-90`. Retain floor (0.05) sits at half the birth value (0.1) and the decay timestamp never resets, so a fresh prototype crosses the floor in ~0.6s. 22h run stored 0 prototypes. (Evidence favors "never born" via an unsatisfied gate over "born then culled," but either way the loop accumulates nothing.) Fix: raise birth stability above the floor / widen reinforcement bin / fix the non-resetting timestamp.
- **Recall store memorizes the live probe-perturbed offset, not the committed correction** — `viewport.rs:4693-4710`; `vocal_servo.rs:98-109`. `store()` runs every tick using raw per-tick fit (bypassing the servo's own EMA), so a lucky-noise probe sample gets memorized as "the correction that worked" and is sticky. Fix: store the committed offset, gated to commit events.
- **Efficacy EMA steps K times per tick on batched pairs while decay sees `elapsed=0`** — `viewport.rs:4211-4227,4384-4415`; `learned_bias.rs:289-297`. Within-tick batches over-weight efficacy and bypass forgetting, violating the documented ~10s window. Maturity has a 1s gate; efficacy doesn't. Fix: rate-limit the efficacy step like maturity, or aggregate same-tick pairs.

**Persistence**
- **Vocal skill-memory writes are non-atomic bare `fs::write`** — `body.rs:613,621,627,735,741,747` (+ `merge_artifacts.rs:57,77,97`). `phoneme_memory.bin` / `vocal_memory.bin` / `vocal_target_recall.bin` truncate-then-write with no tmp+rename; a kill (deploy/rebuild/OOM) in the window leaves a truncated file that silently deserializes to empty — the exact reset the in-code comments forbid. Written every 1s. Fix: reuse the existing atomic checkpoint helper.
- **`read_snapshot` has no length/EOF validation** — `snapshot.rs:740-769`, consumed at `body.rs:384` in `main()` (outside `catch_unwind`). A magic+version-matching but truncated file panics the body at boot; combined with the no-op panic hook (`body.rs:205`) it dies silently with no stderr. Realistic triggers: layout change without version bump, disk-full, interrupted deploy copy. Fix: bounds-safe `read_exact`-style reader returning `Err`; `body.rs` already handles `unwrap_or_default()`.
- **Decay-clock timestamps not persisted → suppression/efficacy/familiarity force-relaxed to ~0 every restart, while maturity is restored** — `learned_bias.rs:99-114,414-423`; `body.rs:225-231`. `now_us()` is wall-clock-since-1970, so the first post-boot decay multiplies by `0.5^(huge)` ≈ 0. The persisted scalar values are immediately wiped; maturity ratchets up untouched. Direct mechanism of "maturity high, efficacy stuck low" under frequent deploys. Fix: anneal maturity on restart too, or persist `last_clean_shutdown_us` for proportional relaxation.
- **All checkpoint + shutdown writes swallow I/O errors (`let _ =`)** — `body.rs:542,596-627,716-747`; `merge_artifacts.rs:57,77,97`. ENOSPC / read-only FS / rename failure → body keeps running and exits 0 as if persisted. The snapshot path *has* a fault counter; the durable checkpoints (which matter more for "死后保留") do not. Fix: increment an observable fault counter parallel to `snap_fail_count`.

**Lifecycle**
- **Mainline feeder has zero convergence/stop detection** — `nursery_feeder.mjs:535-562`. Infinite curriculum; `readVocalEfficacy`/`readMaturity` feed only log strings, never the loop or harvest gate. Once plateaued it keeps burning compute and harvesting a saturated body into master every 120s. Fix: gate harvest on efficacy-improved-since-last; add a plateau watchdog.
- **`sandbox_runner` appends to `in.frames` forever** — `sandbox_runner.rs:45-89`; reader `live_byte_carrier.rs:36-52` only advances an offset, never truncates. 30-min run already left a 6.7 MB file; 22h ≈ ~280 MB. (Dominant driver is the echo path, not the every-2s reseed.) Fix: rotate/truncate once the consumer offset passes a threshold, like `out.frames` already does.
- **Feeder spawns body + sandbox_runner outside bwrap** — `nursery_feeder.mjs:474,479` vs visitor-only `wrapBodyCmd` at `server.mjs:118-138`. The long-lived mainline (which seeds every visitor branch) runs unconfined, breaking the stated uniform sandbox posture. (Input is trusted synthetic curriculum, so not remotely triggerable — defense-in-depth gap, not RCE.) Fix: run the mainline under bwrap too, or document the exemption + confine via low-priv user.

**Body→gateway parser**
- **Stream parser can desync permanently on a corrupt `n`** — `server.mjs:707-738`. Tags 0x02/0x04 trust an unbounded length and `break` (don't resync); `vocalBuf` grows unbounded. Latent only — the transport is a reliable local Unix stream with a single well-formed writer, so there's no realistic in-band trigger today. Fix: cap `n`/`w*h` to protocol maxima, resync on overflow, cap buffer size.

---

## LOW (6)

- **XOR "cipher" ships its 4-byte key in cleartext frame 0** — `security.mjs:127-157`, `server.mjs:638-642`, `Commune.tsx:141-146`. Zero confidentiality against the threat its comment names; per-frame CPU/alloc over 230 KB for no benefit. Real protection is WSS/TLS (acknowledged in the comment). Fix: drop it, or stop calling it a security layer and never send the key in-band. *(Appears twice in the findings; same defect.)*
- **`clientIp()` trusts `cf-connecting-ip`/`x-forwarded-for` with no trusted-proxy check** — `server.mjs:237-242`. Latent only: the gateway binds `127.0.0.1` and CF overwrites the header, so not remotely exploitable today; becomes live if rebound to `0.0.0.0`. Fix: derive IP from a trusted hop / allowlist the tunnel source.
- **Per-AI memory KV is effectively dead** — `server.mjs:840-856`, `memory_store.mjs:20-23`. Only `_meta` (a visit counter) is ever written; the `/memory` PUT route has no caller (the body has no HTTP client, the SPA doesn't write it). "Cross-session AI memory" is cosmetic. Fix: harvest body-side summaries into the KV on disconnect, or stop advertising the feature.
- **`social_response_rate` is write-only telemetry** — `phoneme_memory.rs:141,507-520`. Documented as "the seed of a name/word" but read by no selection/naming logic (the consumer was intentionally de-scoped in a 2026-04-20 cleanup). Plus a dead `_acoustic` param. Fix: wire it into proto selection or demote the doc claim to "telemetry-only."
- **`tick_vocal_imitation_loop` orphaned by the 改造6 refactor** — `viewport.rs:5045`; live ticker `body.rs:461-467` never calls it. The runtime bug it caused (n_prototypes stuck at 0) was already salvaged into `tick_vocal_memory`; what remains is a dead entry point + a stale source-text-scan guard (`curriculum.rs:5003`) that's inert telemetry. Fix: delete the function, replace the string-scan with a behavioral assertion.
- **`credit_response` comment misdescribes its EMA** — `phoneme_memory.rs:513-519`. Claims "decays toward 0 on every call"; it's a plain EMA toward confidence, called only on observed responses, so it can never fall. The function's own docstring contradicts the inline comment. Fix: correct the comment (or add real periodic decay).
- **Doc line-anchors in `vocal-language-upgrade-plan-v1.md:25` are stale** — cite `viewport.rs:3970,4231,4671`, which now point at unrelated code; real entry points are `begin_voicing`/`adjust_voicing`. Fix: anchor to symbol names, not line numbers.

---

## SYSTEMIC THEMES
These are the patterns the owner is actually feeling. Every one is supported by 3+ confirmed defects:

1. **Things that run but don't accumulate.** vocal_memory born-then-erased (#medium), efficacy decay-clock zeroed every restart (#medium), the feeder never converging (#medium), language bindings that only decay (#11), `social_response_rate` feeding nothing (#low). The system *looks* alive — counters climb, ticks fire — but the durable state nets to zero. This is the engine behind "skill memories static, maturity decoupled from competence."

2. **Data built then discarded.** The snapshot serializes rich B1 relation rules that the loader drops (#10); `merge_pool` computes a `score` then throws it away (#medium); `read_snapshot` parses the whole substrate and keeps one field. Expensive structure is faithfully written and then ignored on the path that matters.

3. **The distributed pipeline is a no-op end-to-end.** The feeder clobbers the merge (#1), the harvest gate rarely passes (#medium), the merge dilutes to nothing (#medium), maturity contaminates the merge (#3), and the production env var that enables it isn't even in the committed unit (#medium). Five independent defects all point the same way: visitor learning does not reach master, and master barely moves when it does.

4. **Live comms silently drops/stalls data.** The 230 KB uplink overruns the tunnel (#medium), 1006 drops dead-end the user (#6), the body→gateway parser can wedge forever (#low). The black-screen fix addressed one half of one of these.

5. **Persistence is non-atomic and silent on failure.** Three vocal files use bare `fs::write` (#medium), checkpoint errors are swallowed (#medium), `read_snapshot` panics silently on truncation (#medium), the harvest copy can read half-written files (#medium). When something goes wrong on disk, nothing tells you — you find out at the next restart when the learning is gone.

6. **Security controls that look present but aren't.** Auth disabled (#2), sandbox fails open (#4), XOR is theater (#low), cookies non-revocable (#5), DB ports unhardened (#8), admin compare non-constant-time (#medium). The architecture *documents* a strong posture; the runtime quietly doesn't enforce most of it.

The unifying meta-pattern: **the code is honest in its comments and dishonest in its behavior.** Comments say "must survive death," "no email on disk," "security layer," "does not saturate" — and in each case the behavior is the opposite. The owner is reading intent and feeling it not match.

---

## FIX FIRST
**The mainline feeder clobbering the merged master (#1).** It is the keystone: it single-handedly nullifies the entire distributed-training subsystem (the project's core thesis), it is live in production right now, and it directly produces the most visible symptom — "skill memories are static, nothing accumulates." Make `merge_artifacts` the sole writer of `master_*.bin` and route feeder output through the pool. Fixing it also unblocks the value of fixing #3 (maturity contamination) and #medium-merge-dilution, which are otherwise masked by the clobber.

(Closely behind, and cheaper: #2 auth-disabled + #4 sandbox-fail-open are a one-line-each posture regression on a public service. If "feeling off" includes security unease, do these in the same sitting.)

---

## VERDICT
**The owner's instinct is correct, and more specifically correct than a vague unease deserves.** This is not paranoia and it is not one bug. There is a real, reproducible reason the system feels like it isn't learning: the distributed-training loop is a no-op in steady state (#1), language grounding only decays (#11), competence scalars are wiped every restart while maturity ratchets up (#3, #medium decay-clock), and durable B1 state is thrown away on load (#10). Independently, the live service is running with auth off and its sandbox fail-open. None of these crash; all of them are quiet — which is exactly why it "feels off" rather than "is obviously broken." The codebase is well-instrumented and self-aware in its comments, which makes the gap between stated intent and actual behavior the most reliable signal that something is wrong. It is.