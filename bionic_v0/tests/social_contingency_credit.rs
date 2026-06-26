//! TDD for the social-contingency credit axis (turn-taking → developmental clock).
//!
//! These exercise the PURE estimator (`SocialContingencyState`) — the honest core
//! that decides whether "I vocalised → something answered" earns developmental
//! credit. The viewport/body wiring (which feeds the returned improvement into
//! `advance_maturity_by_progress`) is a thin call layer over this.
//!
//! Cases map 1:1 to the adversarial review's required tests (a)–(g).

use bionic_v0::social_contingency::{
    SocialContingencyState, SOCIAL_CONTINGENCY_CONSOLIDATE_MIN,
};

/// Drive `n` ticks of a fixed (recently_voiced, external_event) regime, summing
/// the credited improvement.
fn drive(state: &mut SocialContingencyState, regime: &[(bool, bool)], reps: usize) -> f64 {
    let mut total = 0.0;
    for _ in 0..reps {
        for &(rv, ext) in regime {
            total += state.update(rv, ext);
        }
    }
    total
}

/// (a) Alone — the body voices but no external voice ever answers, and the room
/// is quiet. ΔP must stay 0: maturity earns nothing socially when solitary.
#[test]
fn alone_earns_no_social_credit() {
    let mut s = SocialContingencyState::default();
    // Half the ticks are "recently voiced", none have an external event.
    let credited = drive(&mut s, &[(true, false), (false, false)], 500);
    assert_eq!(credited, 0.0, "solitary babble must earn no social credit");
    assert_eq!(s.contingency(), 0.0);
}

/// (b) Genuine turn-taking — an external voice reliably answers within the window
/// and the room is otherwise quiet. ΔP rises → positive, bounded credit accrues.
#[test]
fn contingent_answer_earns_bounded_credit() {
    let mut s = SocialContingencyState::default();
    // Post-vocal ticks see the answer; ambient ticks are quiet.
    let credited = drive(&mut s, &[(true, true), (false, false)], 500);
    assert!(credited > 0.0, "contingent answers must earn credit, got {credited}");
    // Bounded: total credited improvement is the rise of a ΔP that lives in [0,1].
    assert!(credited <= 1.0, "credit must be bounded by ΔP∈[0,1], got {credited}");
    assert!(s.contingency() > 0.5, "ΔP should approach 1 under perfect contingency");
}

/// (c) Self-echo — the body's own return is classified self (external_event=false),
/// so it never touches the post-vocal response rate. No social credit.
#[test]
fn self_echo_earns_no_social_credit() {
    let mut s = SocialContingencyState::default();
    // Recently voiced and "sound present", but it is self → external_event=false.
    let credited = drive(&mut s, &[(true, false)], 1000);
    assert_eq!(credited, 0.0, "self-echo must not be credited as a social answer");
}

/// (f) The F3 cheat — constant ambient noise after AND independent of voicing.
/// Both conditionals saturate equally → ΔP ≈ 0 → no maturity. This is the case
/// the naive co-occurrence wiring would have FAILED (noise matures the body).
#[test]
fn constant_ambient_noise_earns_no_credit() {
    let mut s = SocialContingencyState::default();
    let credited = drive(&mut s, &[(true, true), (false, true)], 2000);
    assert!(
        credited < 1e-9,
        "constant ambient noise must not mature the body, got {credited}"
    );
    assert!(s.contingency() < 0.05, "ΔP must collapse under noise, got {}", s.contingency());
}

/// (e) Plateau — once a real contingency is mastered, repeating it earns nothing
/// further (improvement → 0). No runaway from re-observing the same skill.
#[test]
fn mastered_contingency_plateaus() {
    let mut s = SocialContingencyState::default();
    let first = drive(&mut s, &[(true, true), (false, false)], 800);
    assert!(first > 0.0);
    // Keep the SAME regime: ΔP is already at its high-water-mark → no new credit.
    let more = drive(&mut s, &[(true, true), (false, false)], 800);
    assert!(more < 1e-6, "a mastered contingency must not keep crediting, got {more}");
}

/// (consolidation, F2) — before each conditional has CONSOLIDATE_MIN samples, even
/// a strong apparent contingency credits nothing: a green EMA cannot ratchet.
#[test]
fn no_credit_before_consolidation() {
    let mut s = SocialContingencyState::default();
    // Feed fewer than the minimum samples on each side, perfectly contingent.
    let n = (SOCIAL_CONTINGENCY_CONSOLIDATE_MIN as usize).saturating_sub(1);
    let mut credited = 0.0;
    for _ in 0..n {
        credited += s.update(true, true);
        credited += s.update(false, false);
    }
    assert_eq!(credited, 0.0, "no credit until both conditionals are consolidated");
    // The very next pair crosses the threshold and may begin crediting.
    let next = s.update(true, true) + s.update(false, false);
    assert!(next >= 0.0);
}

/// (g, F5) — a fresh/merged body inherits no social high-water-mark. `Default` IS
/// the lineage reset (state is runtime-only, never deserialized/merged), so a new
/// body must credit from zero, never silently sit on an inherited high baseline.
#[test]
fn fresh_state_resets_high_water_mark() {
    // Earn a high contingency on one "lineage".
    let mut earned = SocialContingencyState::default();
    let _ = drive(&mut earned, &[(true, true), (false, false)], 800);
    assert!(earned.contingency() > 0.5);

    // A fresh body (the reset) starts at zero contingency and can earn credit
    // from scratch — it does NOT inherit the prior lineage's high-water-mark.
    let mut fresh = SocialContingencyState::default();
    assert_eq!(fresh.contingency(), 0.0, "fresh body must start at zero ΔP");
    let credited = drive(&mut fresh, &[(true, true), (false, false)], 800);
    assert!(credited > 0.0, "fresh body must be able to earn from zero, got {credited}");
}
