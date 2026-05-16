use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("bionic_v0 has a repository parent")
        .to_path_buf()
}

fn read_repo_file(path: &str) -> String {
    let full_path = repo_root().join(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", full_path.display()))
}

fn assert_absent(source_name: &str, source: &str, forbidden: &[&str]) {
    let present: Vec<&str> = forbidden
        .iter()
        .copied()
        .filter(|term| source.contains(term))
        .collect();

    assert!(
        present.is_empty(),
        "{source_name} still contains contraction-forbidden architecture terms: {present:?}"
    );
}

#[test]
fn evolution_priors_no_domain_specific_bias_axes_or_feature_names() {
    let source = read_repo_file("bionic_v0/src/evolution_priors.rs");

    assert_absent(
        "evolution_priors.rs",
        &source,
        &[
            "CodeNativeFeatures",
            "DigitalStructureFeatures",
            "GeneralDisruptionFeatures",
            "VisualPrimitiveFeatures",
            "AcousticPrimitiveFeatures",
            "SpatialDistanceFeatures",
            "ActionConsequenceFeatures",
            "CommunicationLoopFeatures",
            "EpistemicTensionFeatures",
            "CrossModalBindingFeatures",
            "CodeStream",
            "DecoderProjection",
            "VisualPrimitive",
            "AcousticPrimitive",
            "ActionConsequence",
            "SpatialDistance",
            "CommunicationLoop",
            "CrossModalBinding",
            "glyph_alignment",
            "visual_attention",
            "acoustic_attention",
            "communication_retention",
            "epistemic_tension",
            "cross_modal_binding",
        ],
    );
}

#[test]
fn probe_law_and_body_no_discrete_probe_trigger_function_names() {
    let probe_law = read_repo_file("bionic_v0/src/probe_law.rs");
    let body = read_repo_file("bionic_v0/src/bin/body.rs");
    let combined = format!("{probe_law}\n{body}");

    assert_absent(
        "probe_law.rs/body.rs",
        &combined,
        &[
            "tick_surprise_trigger",
            "tick_attention_trigger",
            "tick_relation_instability_trigger",
            "Surprise-driven trigger",
            "Attention-rebalance trigger",
            "Relation-instability trigger",
        ],
    );
}

#[test]
fn adaptive_stability_no_named_route_kinds_or_phases() {
    let source = read_repo_file("bionic_v0/src/adaptive_stability.rs");

    assert_absent(
        "adaptive_stability.rs",
        &source,
        &[
            "AdaptiveRouteKind",
            "AdaptiveRoutePhase",
            "Attention",
            "ActionConsequence",
            "VocalPrototype",
            "Stable",
            "Bypassing",
            "Relearning",
            "FastPathActive",
        ],
    );
}

#[test]
fn code_ingress_no_ascii_or_glyph_projection_terms() {
    let source = read_repo_file("bionic_v0/src/code_ingress.rs");

    assert_absent(
        "code_ingress.rs",
        &source,
        &[
            "ASCII",
            "ascii",
            "Glyph",
            "glyph",
            "CODE_DECODER_ASCII_SHAPE",
        ],
    );
}

#[test]
fn global_architecture_map_is_not_stale_snapshot_v41() {
    let lib = read_repo_file("bionic_v0/src/lib.rs");
    let source =
        read_repo_file("docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md");

    let version_line = lib
        .lines()
        .find(|line| line.contains("pub const SNAPSHOT_VERSION"))
        .expect("snapshot version constant");
    let version = version_line
        .split('=')
        .nth(1)
        .expect("version rhs")
        .trim()
        .trim_end_matches(';');

    assert!(
        source.contains(&format!("snapshot v{version}")),
        "architecture map must mention current snapshot v{version}"
    );

    assert_absent(
        "global architecture map",
        &source,
        &["snapshot v41", "integrated v41 layout"],
    );
}
