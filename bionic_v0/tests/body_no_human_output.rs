//! Cleanup spec 2026-04-20 §4.5: the body process must not emit
//! human-readable text. All visible output must come from the observer
//! reading snapshot facts. Direct prints inside the body cross the
//! research/body separation red line (Prime Directive 推论1 + 推论2).
//!
//! This guard scans the *runtime* code of every body-owned module. Unit tests
//! live in a trailing `#[cfg(test)] mod tests` and may legitimately use
//! println!/format!/expect! for diagnostics, so they are stripped before the
//! scan (see `strip_trailing_test_module`).

/// Blank out everything from the first `#[cfg(test)]` line to EOF so only
/// body-runtime code is inspected. Every body module in this crate places its
/// test module last, so this strips exactly the test code. Lines are blanked
/// (not removed) so reported line numbers still match the original file.
fn strip_trailing_test_module(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_test = false;
    for line in src.lines() {
        if !in_test && line.contains("#[cfg(test)]") {
            in_test = true;
        }
        if !in_test {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[test]
fn body_owned_sources_contain_no_human_readable_diagnostics() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Body-owned sources = the body binary + every lib module that runs inside
    // the always-running body process (imported by `body.rs` or ticked from its
    // main loop). Observer/research tooling (observer*.rs, surface_*.rs,
    // post_effect.rs, curriculum*.rs, capability_curriculum_governor.rs,
    // five_phase_life.rs) is intentionally excluded — it is allowed to render
    // human-readable output.
    let sources = [
        // --- body binary ---
        "src/bin/body.rs",
        // --- core loop / prediction / state ---
        "src/predictor.rs",
        "src/highlevel.rs",
        "src/viewport.rs",
        "src/clock.rs",
        "src/snapshot.rs",
        "src/action_consequence.rs",
        "src/object_high_layer.rs",
        "src/prediction_pressure_field.rs",
        "src/vpc_cross_layer.rs",
        // --- sensory adapters / IO (body side) ---
        "src/adapter.rs",
        "src/net_adapter.rs",
        "src/vib.rs",
        "src/phot_camera.rs",
        "src/phot_gaussian.rs",
        "src/live_byte_carrier.rs",
        "src/live_byte_closure.rs",
        "src/auditory_visual.rs",
        // --- vocal path (where the acoustic_dbg.log leak lived) ---
        "src/vocal_adapter.rs",
        "src/vocal_synthesis.rs",
        "src/vocal_organ.rs",
        "src/vocal_reflex.rs",
        "src/vocal_drive_field.rs",
        "src/vocal_target.rs",
        "src/vocal_imitation.rs",
        "src/vocal_memory.rs",
        "src/vocal_self_analysis.rs",
        "src/phoneme_memory.rs",
        "src/virtual_neck.rs",
        "src/efference_copy.rs",
        "src/language_binding.rs",
        // --- drives / "soul" / self model ---
        "src/vital_core.rs",
        "src/vital_env.rs",
        "src/dual_drive.rs",
        "src/self_model.rs",
        "src/self_development.rs",
        "src/familiar_objects.rs",
        "src/learned_bias.rs",
        "src/evidence_tension.rs",
        "src/adaptive_stability.rs",
        "src/allostatic_stability.rs",
        "src/life_closure_producers.rs",
        "src/active_surface.rs",
        "src/probe_law.rs",
        "src/mark_disturbance.rs",
    ];
    let forbidden = [
        "println!",
        "eprintln!",
        "dbg!",
        "writeln!",
        "write!(",
        "panic!(\"",
        "expect(",
        "format!(\"",
        "let _ = reg.register",
    ];

    let mut violations = Vec::new();
    for source in sources {
        let path = std::path::Path::new(manifest_dir).join(source);
        let raw = std::fs::read_to_string(&path).unwrap_or_else(|err| {
            panic!("failed to read {}: {err}", path.display());
        });
        let contents = strip_trailing_test_module(&raw);

        for (line_idx, line) in contents.lines().enumerate() {
            for marker in forbidden {
                if line.contains(marker) {
                    violations.push(format!("{source}:{} contains {marker}", line_idx + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "body-owned sources must not contain human-readable diagnostics:\n{}",
        violations.join("\n")
    );
}

#[test]
fn body_exposes_solitary_low_power_runtime_profile_without_text_diagnostics() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let body_path = std::path::Path::new(manifest_dir).join("src/bin/body.rs");
    let clock_path = std::path::Path::new(manifest_dir).join("src/clock.rs");
    let body = std::fs::read_to_string(&body_path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", body_path.display());
    });
    let clock = std::fs::read_to_string(&clock_path).unwrap_or_else(|err| {
        panic!("failed to read {}: {err}", clock_path.display());
    });

    assert!(
        body.contains("--solitary-low-power"),
        "body must accept an explicit long-run low-power launch flag"
    );
    assert!(
        body.contains("BodyRuntimeProfile"),
        "body low-power behavior should be centralized in one runtime profile"
    );
    assert!(
        clock.contains("spawn_master_clock_hz"),
        "master clock rate must be configurable for low-power runs"
    );
}
