# Parallel Focus Field + Active Aperture Echo v0 Design

## Boundary

This design adds a numeric active-surface substrate over the existing
prediction pressure field. It does not add a thought module, desire module,
goal module, mind module, consciousness module, task scheduler, request/response
layer, question/answer layer, naming gate, token layer, or concept allocator.

"Subconscious", "conscious", "focus", "sudden idea", and "thinking" remain
external design metaphors. Inside the body, this slice only manipulates numeric
pressure traces, temporal persistence, resonance, aperture share, echo share,
closure support, write risk, and reentry evidence.

## Core Correction

The active surface must not treat focus as a program task that can be interrupted
and discarded. Multiple relationship cores may remain active in parallel.

The execution constraint is narrower:

- many focus traces may coexist,
- one trace occupies the active aperture,
- one secondary trace may contribute bounded echo pressure,
- non-aperture traces remain as residue and may reenter later,
- closure lowers the aperture trace without deleting the parallel field.

This preserves the life-substrate interpretation: waves compete, overlap,
decay, resonate, and reenter. They are not queued jobs.

## Data Model

`ActiveSurfaceState` lives in `Viewport` beside `prediction_pressure_field`.

It contains:

- `traces`: parallel focus traces projected from pressure-field waves,
- `aperture_trace_index`: the trace currently occupying the execution aperture,
- `aperture_pressure`: primary probe drive pressure,
- `echo_trace_index`: the secondary trace contributing echo pressure,
- `echo_pressure`: bounded auxiliary pressure,
- counters for aperture switches, echo presence, reentry, displacement, retained
  residue, closure reductions, and updates.

`ActiveFocusTrace` contains only numeric/time/index fields:

- source kind,
- slot index,
- edge index,
- pressure,
- persistence,
- resonance,
- closure affordance,
- write risk,
- aperture share,
- echo share,
- residue strength,
- last update time.

## Refresh Rule

`Viewport::refresh_active_surface(now_us)` reads
`prediction_pressure_field` and writes `active_surface`.

For each prediction wave:

1. Decay existing trace pressure and residue.
2. Project matching wave pressure into a focus trace.
3. Rank traces by focus score.
4. Select one aperture trace with inertia and a switch margin.
5. Select one echo trace from the next strongest non-aperture trace.
6. Bound echo pressure so it cannot exceed `aperture_pressure * 0.25`.
7. Retain non-aperture traces as residue unless their pressure and residue fall
   below a small floor.

The refresh rule never clears the whole field just because another focus enters
the aperture.

## Closure Rule

`Viewport::apply_active_surface_closure(feedback)` reduces only the aperture
trace pressure and residue according to numeric feedback. Other traces remain
available for later reentry.

Closure is evidence of local pressure reduction, not deletion.

## Probe Law Coupling

Probe law should read the active surface when it exists:

```text
surface_drive = aperture_pressure + echo_pressure * 0.35
```

Then clamp to `[0,1]` and fold it into the existing pressure drive. The raw
prediction field remains a fallback when no active-surface trace exists.

Echo pressure cannot independently become the aperture. It only modulates the
current execution pressure.

## Body Refresh Order

The body global refresh order becomes:

```text
active_reasoning_gap
prediction_pressure_field
active_surface
probe_law
probe_output
snapshot
```

`active_surface` must run after pressure field hydration and before probe law.

## Observer Evidence

`curriculum_report` receives:

```text
--active-surface-aperture-echo
```

It prints compact numeric evidence only:

- parallel focus trace count,
- aperture count,
- echo presence,
- echo bound,
- dominant trace in aperture,
- non-aperture retention,
- suppressed focus reentry,
- closure reduction without deletion,
- event displacement with residue retention,
- probe drive with echo,
- bounded aperture switches,
- body source order guard,
- principle guard.

## Acceptance

The slice is acceptable only if:

- at least three traces can coexist,
- aperture count is exactly one when pressure exists,
- echo is present under competing pressure and remains bounded,
- a non-aperture trace is retained,
- a suppressed trace can reenter after quiet rehydration,
- closure reduces the aperture trace without deleting all traces,
- a stronger event can displace the aperture while the old trace remains residue,
- probe drive tracks aperture plus bounded echo,
- aperture switches remain bounded under stress,
- touched sources avoid rejected architecture terms and body human-readable
  output.
