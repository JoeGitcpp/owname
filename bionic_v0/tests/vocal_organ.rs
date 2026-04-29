use bionic_v0::vocal_organ::{VocalOrganPhase, VocalOrganState};

#[test]
fn vocal_organ_tracks_continuous_vowel_like_parameters() {
    let mut st = VocalOrganState::default();
    st.begin_voicing(1_000, 0.6, 0.4, 0.7);
    assert_eq!(st.phase, VocalOrganPhase::Active);
    assert!(st.breath_drive > 0.0);
}
