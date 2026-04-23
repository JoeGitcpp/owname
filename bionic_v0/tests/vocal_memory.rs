use bionic_v0::vocal_memory::VocalMemoryState;

#[test]
fn repeated_success_sediments_a_reusable_vocal_prototype() {
    let mut st = VocalMemoryState::default();
    for i in 0..5 {
        st.record_success(1_000 + i, 0.55, 0.40, 0.70);
    }
    assert!(st.prototypes.len() >= 1);
    assert!(st.prototypes[0].stability > 0.0);
}