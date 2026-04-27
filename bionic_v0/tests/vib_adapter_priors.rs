//! Verify VibAdapter can be constructed with a custom BiasPriors
//! and that it stores the reference.

use bionic_v0::adapter::VibAdapter;
use bionic_v0::evolution_priors::BiasPriors;
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

#[test]
fn test_vib_adapter_with_priors_stores_arc() {
    let vp = Arc::new(Viewport::new());
    let mut priors = BiasPriors::default();
    priors.disruption_surprise_mul = 9.99;

    let priors_arc = Arc::new(priors);
    let adapter = VibAdapter::with_priors(&vp, priors_arc.clone());

    assert_eq!(adapter.priors().disruption_surprise_mul, 9.99);
}

#[test]
fn test_vib_adapter_default_uses_default_priors() {
    let vp = Arc::new(Viewport::new());
    let adapter = VibAdapter::new(&vp);
    let default_mul = BiasPriors::default().disruption_surprise_mul;
    assert_eq!(adapter.priors().disruption_surprise_mul, default_mul);
}
