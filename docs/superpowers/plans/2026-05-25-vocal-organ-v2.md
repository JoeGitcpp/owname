# Vocal Organ v2 — 共振峰合成升级实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 vocal organ 从 3 参数正弦波合成器升级到 8 参数共振峰声源-滤波器模型，让身体能产生可辨识的元音轮廓和辅音过渡。

**Architecture:** 扩展 `VocalOrganState` 参数集，替换 `vocal_adapter.rs` 中的正弦波合成为共振峰滤波器，扩展 `vocal_imitation` 的 mel→参数映射。不做语义标注，不引入音素/词语/语言概念。最高原则：新参数仍是纯数值物理维度（频率 Hz、带宽 Q、能量比），不是元音标签。

**Tech Stack:** Rust, existing bionic_v0 vocal pipeline, cpal audio output.

**最高原则合规：**

| 原则 | 合规 |
|------|------|
| 无语义标签 | 参数名是 F1/F2/F3 (Hz)、voiced_ratio、breath_noise_ratio — 物理量，非元音名 |
| 身体/观察分离 | 合成器升级在 body-side 物理层；observer 不变 |
| 主动输出 | 参数由 substrate 驱动，不由外部控制 |
| 硅基生命 | 声源-滤波器模型 = 数字声道模拟，等价于物理声道 |

---

## 文件变更

| 文件 | 变更 |
|------|------|
| `bionic_v0/src/vocal_organ.rs` | 扩展参数集 3→8 |
| `bionic_v0/src/vocal_adapter.rs` | 替换合成引擎 |
| `bionic_v0/src/vocal_imitation.rs` | 扩展 mel→参数映射 |
| `bionic_v0/tests/vocal_reflex.rs` | 扩展测试 |
| `bionic_v0/tests/vocal_imitation.rs` | 扩展测试 |

---

## Task 1: 扩展 VocalOrganState 参数集

**Files:** Modify `bionic_v0/src/vocal_organ.rs`, Test `bionic_v0/tests/vocal_reflex.rs`

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn vocal_organ_v2_has_formant_and_envelope_parameters() {
    let state = bionic_v0::vocal_organ::VocalOrganState::default();

    // New parameters must exist and be finite
    assert!(state.formant_f1.is_finite());
    assert!(state.formant_f2.is_finite());
    assert!(state.formant_f3.is_finite());
    assert!(state.voiced_ratio.is_finite());
    assert!(state.breath_noise_ratio.is_finite());
    assert!(state.envelope_attack.is_finite());
    assert!(state.envelope_decay.is_finite());
    assert!(state.envelope_sustain.is_finite());
}
```

- [ ] **Step 2: Run test to verify FAIL**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_reflex vocal_organ_v2 2>&1 | tail -3
```

Expected: FAIL — fields do not exist.

- [ ] **Step 3: Add fields to VocalOrganState**

```rust
pub struct VocalOrganState {
    // v1 parameters (kept for backward compatibility)
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub noise_mix: f64,
    // v2 formant parameters
    pub formant_f1: f64,    // Hz, typical range 200-1000
    pub formant_f2: f64,    // Hz, typical range 500-3000
    pub formant_f3: f64,    // Hz, typical range 1500-4000
    pub voiced_ratio: f64,  // 0=breath, 1=fully voiced
    pub breath_noise_ratio: f64,  // turbulent noise mix
    // v2 envelope
    pub envelope_attack: f64,   // 0-1, normalized attack time
    pub envelope_decay: f64,    // 0-1, normalized decay time
    pub envelope_sustain: f64,  // 0-1, sustain level
    pub phase: VocalOrganPhase,
    pub last_update_t_us: u64,
}
```

- [ ] **Step 4: Add begin_voicing_v2 method**

```rust
impl VocalOrganState {
    pub fn begin_voicing_v2(
        &mut self, now_us: u64,
        pitch: f64, f1: f64, f2: f64, f3: f64,
        voiced: f64, noise: f64,
        attack: f64, decay: f64, sustain: f64,
    ) {
        self.pitch_tension = pitch.clamp(0.0, 1.0);
        self.breath_drive = 1.0;
        self.formant_f1 = f1.clamp(150.0, 1200.0);
        self.formant_f2 = f2.clamp(400.0, 3500.0);
        self.formant_f3 = f3.clamp(1200.0, 4500.0);
        self.voiced_ratio = voiced.clamp(0.0, 1.0);
        self.breath_noise_ratio = noise.clamp(0.0, 1.0);
        self.envelope_attack = attack.clamp(0.0, 1.0);
        self.envelope_decay = decay.clamp(0.0, 1.0);
        self.envelope_sustain = sustain.clamp(0.0, 1.0);
        self.phase = VocalOrganPhase::Active;
        self.last_update_t_us = now_us;
    }
}
```

- [ ] **Step 5: Run test**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_reflex vocal_organ_v2 2>&1 | tail -3
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add bionic_v0/src/vocal_organ.rs bionic_v0/tests/vocal_reflex.rs
git commit -m "feat: add v2 formant and envelope parameters to vocal organ"
```

---

## Task 2: 替换合成引擎为正弦波+共振峰

**Files:** Modify `bionic_v0/src/vocal_adapter.rs`, Test `bionic_v0/tests/vocal_reflex.rs`

- [ ] **Step 1: Write failing test for formant output**

```rust
#[test]
fn vocal_organ_v2_formants_produce_distinct_spectra() {
    use bionic_v0::vocal_organ::VocalOrganState;
    use bionic_v0::vocal_adapter::sample_vocal_waveform_v2;

    // /a/-like: high F1, high F2
    let mut a_like = VocalOrganState::default();
    a_like.begin_voicing_v2(0, 0.5, 800.0, 1200.0, 2500.0, 1.0, 0.0, 0.1, 0.5, 0.8);
    let a_wave = sample_vocal_waveform_v2(&a_like, 480);

    // /i/-like: low F1, high F2
    let mut i_like = VocalOrganState::default();
    i_like.begin_voicing_v2(0, 0.5, 300.0, 2200.0, 3000.0, 1.0, 0.0, 0.1, 0.5, 0.8);
    let i_wave = sample_vocal_waveform_v2(&i_like, 480);

    // Different formants should produce different waveforms
    let diff: f64 = a_wave.iter().zip(i_wave.iter())
        .map(|(a, i)| (a - i).abs()).sum::<f64>() / 480.0;
    assert!(diff > 0.01, "/a/ and /i/ formants should produce distinct waveforms, got diff={}", diff);
}
```

- [ ] **Step 2: Implement sample_vocal_waveform_v2**

In `bionic_v0/src/vocal_adapter.rs`, add:

```rust
pub fn sample_vocal_waveform_v2(organ: &VocalOrganState, n_samples: usize) -> Vec<f64> {
    let mut samples = Vec::with_capacity(n_samples);
    if organ.breath_drive < 0.01 {
        for _ in 0..n_samples { samples.push(0.0); }
        return samples;
    }

    let pitch = 80.0 + organ.pitch_tension * 420.0; // 80-500 Hz
    let f1 = organ.formant_f1.max(100.0);
    let f2 = organ.formant_f2.max(200.0);
    let f3 = organ.formant_f3.max(500.0);

    for i in 0..n_samples {
        let t = i as f64 / n_samples as f64;
        let dur = n_samples as f64;

        // Envelope: ADSR-like
        let attack_samples = organ.envelope_attack * dur * 0.3;
        let decay_samples = organ.envelope_decay * dur * 0.3;
        let sustain_level = organ.envelope_sustain;
        let env = if (i as f64) < attack_samples {
            (i as f64) / attack_samples.max(1.0)
        } else if (i as f64) < attack_samples + decay_samples {
            let d = ((i as f64) - attack_samples) / decay_samples.max(1.0);
            1.0 - d * (1.0 - sustain_level)
        } else {
            sustain_level
        };

        // Glottal source: pulse train + noise
        let phase = (t * pitch).fract();
        let voiced_pulse = if phase < 0.02 {
            (phase / 0.02).sin() * std::f64::consts::PI * 0.5
        } else {
            0.0
        };
        let noise = (t * 44000.0).sin() * (t * 337.0).cos();
        let source = organ.voiced_ratio * voiced_pulse
            + (1.0 - organ.voiced_ratio) * organ.breath_noise_ratio * noise;

        // Formant filter: 3 parallel resonators
        let resonance = |freq: f64, q: f64| -> f64 {
            let w = 2.0 * std::f64::consts::PI * freq / 44100.0;
            let r = (-w / q).exp();
            1.0 / (1.0 - 2.0 * r * w.cos() + r * r)
        };

        let f1_gain = resonance(f1, 8.0);
        let f2_gain = resonance(f2, 12.0);
        let f3_gain = resonance(f3, 15.0);
        let formant_filter = (f1_gain * 1.2 + f2_gain * 0.8 + f3_gain * 0.4) / 3.0;

        let sample = organ.breath_drive * env * source * formant_filter;
        samples.push(sample.clamp(-1.0, 1.0));
    }
    samples
}
```

- [ ] **Step 3: Update next_wave_sample to use v2 path**

In `fill_output_buffer_*` functions, replace `next_wave_sample` call with:

```rust
fn next_wave_sample_v2(vp: &Arc<Viewport>, phase: &mut f64, sample_rate_hz: f64) -> f32 {
    let organ = vp.read_vocal_organ();
    if organ.breath_drive < 0.01 {
        return 0.0;
    }
    // ... use v2 formant synthesis as above, but real-time per-sample
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_reflex 2>&1 | tail -3
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/src/vocal_adapter.rs bionic_v0/tests/vocal_reflex.rs
git commit -m "feat: replace sine-wave synth with formant source-filter model"
```

---

## Task 3: 扩展 vocal imitation 映射

**Files:** Modify `bionic_v0/src/vocal_imitation.rs`, Test `bionic_v0/tests/vocal_imitation.rs`

- [ ] **Step 1: Extend mel→formant mapping**

Vocal imitation 将 mel 频谱特征映射到 organ 参数。扩展映射覆盖新参数。

- [ ] **Step 2: Test — imitation produces full parameter set**

```rust
#[test]
fn vocal_imitation_v2_produces_formant_parameters() {
    // Setup: seed vocal memory with mel contour
    // Tick imitation
    // Assert: organ has non-default formant values
}
```

- [ ] **Step 3: Commit**

```bash
git add bionic_v0/src/vocal_imitation.rs bionic_v0/tests/vocal_imitation.rs
git commit -m "feat: extend vocal imitation to learn formant parameters"
```

---

## Task 4: 全量验证 + 原则守护

- [ ] **Step 1: Full test suite**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml | grep "test result:"
```

- [ ] **Step 2: G1 — 无语义污染**

```bash
grep -r "vowel\|consonant\|phoneme\|syllable\|word\|speech\|language" bionic_v0/src/vocal_*.rs && echo "G1 FAIL" || echo "G1 PASS"
```

- [ ] **Step 3: G5 — 无 body-side println**

```bash
grep -r "println\|eprintln" bionic_v0/src/vocal_*.rs --include="*.rs" | grep -v "tests/" && echo "G5 FAIL" || echo "G5 PASS"
```

- [ ] **Step 4: Commit verification**

```bash
git commit -m "chore: verify vocal organ v2 — all tests pass, principle gates pass"
```
