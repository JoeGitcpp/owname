use bionic_v0::curriculum_material::{
    default_external_curriculum_training_plan, review_external_curriculum_training_plan,
};

#[test]
fn default_external_curriculum_plan_covers_required_material_origins_carriers_and_stages() {
    let plan = default_external_curriculum_training_plan();
    let review = review_external_curriculum_training_plan(&plan);

    assert!(review.material_groups >= 8);
    assert!(review.synthetic_groups >= 3);
    assert!(review.replay_groups >= 1);
    assert!(review.probe_return_groups >= 1);
    assert!(review.live_ingress_groups >= 1);
    assert!(review.carrier_kinds >= 5);
    assert!(review.stage_kinds >= 7);
    assert!(review.reproducible_groups >= 6);
    assert!(review.observer_only_groups >= review.material_groups);
    assert_eq!(review.body_touch_points, 0);
    assert_eq!(review.missing_required_roles, 0);
    assert_eq!(review.principle_review_pass, 1);
}

#[test]
fn external_curriculum_material_registry_uses_indices_not_rejected_ontology() {
    let src = include_str!("../src/curriculum_material.rs");
    let forbidden = [
        "semantic_readiness",
        "SymbolizerState",
        "concept_id",
        "token_id",
        "word_id",
        "object_name",
        "text_understanding",
        "name_gate",
        "meaning_db",
        "dictionary",
        "parser",
        "tokenizer",
        "image_label",
        "voice_label",
        "prompt_answer",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "println!",
        "eprintln!",
    ];

    for term in forbidden {
        assert!(
            !src.contains(term),
            "material registry source contains rejected term: {term}"
        );
    }
}
