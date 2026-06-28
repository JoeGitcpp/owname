# HTM 启发的上下文预测与群体异常 — 软件设计文档 (SDD)

**日期**：2026-06-28
**版本**：v1
**状态**：REVIEW
**前置阅读**：`cognitive-middle-layers-sdd-2026-06-27.md`、`prospection-emergence-scaffold-sdd-2026-06-27.md`、`temporal-credit-horizon-supplement-2026-06-28.md`

---

## 0. 文档定位

这是两个独立特性的联合设计文档，均受 HTM (Hierarchical Temporal Memory, Numenta/Jeff Hawkins) 启发但不照搬 HTM 实现。

**一句话**：给 AR-2 预测器装上上下文记忆（知道「我从哪来」），给系统装上群体级异常信号（知道「整片区域集体偏离预期」），分别填补时间序列预测的上下文盲区和前瞻涌现 E 组件的信号空洞。

### 0.1 两特性关系

| 特性 | 填补的空洞 | 依赖 |
|------|-----------|------|
| **A. Context-Gated AR-4** | AR-2 上下文盲（同一数值在不同轨迹中的预测相同） | 独立，不依赖 B |
| **B. 模态级群体异常信号** | E 组件 15%→更高（缺群体级「世界异常了」信号） | 独立，不依赖 A；接通已有的 convergence_metric.rs |

两者可独立实施、独立测试、独立部署。建议先 B（工作量更小，接通已有代码），后 A。

---

## 1. 特性 A：Context-Gated AR-4 预测器

### 1.1 问题陈述

当前预测器（`predictor.rs:6-129`）是 AR-2 线性模型：

```
pred = a · x₀ + b · x₁ + external_prior
```

- `x₀` = 当前观测，`x₁` = 前一个观测
- 2 个可学习系数 `a, b`（per axis）
- 167 轴 × 2 = **334 个参数**

**上下文盲区**：对 AR-2 而言，`x₀=300, x₁=250`（上升中）和 `x₀=300, x₁=350`（下降中）是完全不同的输入——这没问题。但更长的上下文信息丢失了：

```
序列 A: 100 → 200 → 300 → ???    （加速上升）
序列 B: 400 → 350 → 300 → ???    （减速下降）
```

AR-2 只看到最近两帧 `(300, 200)` vs `(300, 350)`，能区分。但：

```
序列 C: 100 → 200 → 300 → ???    （从低位起步的上升）
序列 D: 500 → 200 → 300 → ???    （大幅回调后的回弹）
```

AR-2 看到的都是 `(300, 200)`——**完全相同的输入产生完全相同的预测**，但合理的预测完全不同（C 应预测 ~400 继续上升，D 应预测 ~250 回归均值）。

HTM 用多个树突段解决这个问题：同一个细胞根据「哪个树突段匹配了」做不同预测。我们不需要完整的树突段机制，但需要让预测器知道「我从哪来」。

### 1.2 设计决策

#### 1.2.1 为什么不直接用 AR-4

简单扩展到 `pred = a·x₀ + b·x₁ + c·x₂ + d·x₃` 增加了历史深度但没有增加**上下文选择性**。4 个系数对所有轨迹都一样——仍然只有一套预测规则。

#### 1.2.2 为什么不用完整的 HTM 树突段

每个 HTM 细胞有 ~128 个树突段，每个段有 ~40 个突触。对 167 轴：167 × 128 × 40 = ~855,000 个突触永久性值。这比整个系统现有参数量大两个数量级，而且 HTM 的二值连接阈值与我们的连续学习范式不兼容。

#### 1.2.3 选定方案：Context-Gated AR-4

**核心思路**：保留 Widrow-Hoff 局部学习，但给每个轴维护 `N_CTX` 套系数，运行时只激活匹配当前上下文的那一套。

```
ctx = quantize_trajectory(x₀, x₁, x₂, x₃) → 桶索引 ∈ [0, N_CTX)
pred = a[ctx]·x₀ + b[ctx]·x₁ + c[ctx]·x₂ + d[ctx]·x₃ + external_prior
```

上下文量化函数将最近 4 帧的轨迹特征映射到离散桶，学习只更新被激活的那套系数——天然局部，不需要 backprop。

### 1.3 上下文量化函数

#### 1.3.1 轨迹特征

从最近 4 个观测 `x₀, x₁, x₂, x₃`（`x₀` 最新）提取 2 个特征：

```
// 一阶差分的符号（方向）
sign_recent = sign(x₀ - x₁)    // +1, 0, -1
sign_older  = sign(x₁ - x₂)    // +1, 0, -1

// 二阶差分（加速度方向）
accel = sign((x₀ - x₁) - (x₁ - x₂))  // +1, 0, -1
```

#### 1.3.2 桶映射

使用 2-bit 编码（去掉 0，只保留正/负）：

```
bit_0 = (x₀ > x₁) as usize      // 最近方向：1=上升, 0=下降/平
bit_1 = (x₁ > x₂) as usize      // 前一方向：1=上升, 0=下降/平

ctx = bit_0 * 2 + bit_1          // → 4 个桶
```

| ctx | bit_1 (旧) | bit_0 (新) | 含义 |
|-----|-----------|-----------|------|
| 0 | 下降 | 下降 | 持续下降 |
| 1 | 下降 | 上升 | 反转上升（谷底） |
| 2 | 上升 | 下降 | 反转下降（峰顶） |
| 3 | 上升 | 上升 | 持续上升 |

**N_CTX = 4**，每轴 4 套 × 4 系数 = 16 参数（vs 当前 2 参数）。
全系统：167 × 16 = **2,672 参数**（vs 当前 334）。

#### 1.3.3 为什么是 4 桶不是 8

8 桶（加入 accel bit）理论上更精确，但：
- 每桶接收训练样本减半 → 收敛更慢
- 新生期（m ≈ 0）数据稀疏，8 桶可能长时间有空桶
- 4 桶覆盖了最重要的区分（方向反转 vs 持续），加速度区分是锦上添花

设计允许未来扩展到 8 桶（改 `N_CTX` 常量 + 加一个 bit），但初始实现用 4。

### 1.4 数据结构变更

#### 1.4.1 Ar2Predictor → ContextGatedPredictor

**文件**：`predictor.rs`

```rust
pub const N_CTX: usize = 4;
pub const N_TAPS: usize = 4;

pub struct ContextGatedPredictor {
    pub axis: usize,
    /// N_CTX sets of N_TAPS coefficients. coeffs[ctx][tap].
    pub coeffs: [[f64; N_TAPS]; N_CTX],
    pub base_lr: f64,
    pub clip: f64,
    pub horizon_us: u64,
    pub pending_pred: Option<PendingPredCg>,
}

pub struct PendingPredCg {
    pub predicted_value: f64,
    pub used_x: [f64; N_TAPS],
    pub used_ctx: usize,
    pub seq_at_prediction: u64,
    pub publish_t_us: u64,
    pub target_t_us: u64,
}
```

#### 1.4.2 上下文量化

```rust
fn quantize_ctx(x: &[f64; N_TAPS]) -> usize {
    let bit_0 = (x[0] > x[1]) as usize;
    let bit_1 = (x[1] > x[2]) as usize;
    bit_0 * 2 + bit_1
}
```

#### 1.4.3 预测

```rust
fn predict(&self, x: &[f64; N_TAPS], prior: f64) -> (f64, usize) {
    let ctx = quantize_ctx(x);
    let c = &self.coeffs[ctx];
    let pred = c[0]*x[0] + c[1]*x[1] + c[2]*x[2] + c[3]*x[3] + prior;
    (pred, ctx)
}
```

#### 1.4.4 学习规则

```rust
fn update(&mut self, ctx: usize, err: f64, x: &[f64; N_TAPS], m: f64, surprise: f64) {
    let eff_lr = self.base_lr * surprise * crate::ar2_lr_scale(m);
    for tap in 0..N_TAPS {
        let d = eff_lr * err * x[tap];
        self.coeffs[ctx][tap] = (self.coeffs[ctx][tap] + d).clamp(-self.clip, self.clip);
    }
    // 其他 N_CTX-1 套系数不更新 — 天然局部学习
}
```

**关键不变量**：只更新被激活桶的系数。非激活桶的系数不衰减、不更新——它们在「自己的上下文」出现时才学习。这和 HTM 树突段的学习规则完全一致（只修改激活段的突触）。

### 1.5 Viewport 接口变更

#### 1.5.1 PredictorState 扩展

**文件**：`viewport.rs:136-156`

```rust
pub struct PredictorState {
    pub a: f64,           // 保留：向后兼容，存 coeffs[last_ctx][0]
    pub b: f64,           // 保留：向后兼容，存 coeffs[last_ctx][1]
    pub surprise: f64,
    pub last_err: f64,
    pub total_updates: u64,
    pub mean_sq_err: f64,
    pub last_ctx: usize,  // 新增：最近使用的上下文桶
}
```

`update_predictor` 方法增加 `ctx: usize` 参数，`a` 和 `b` 字段始终反映最近一次预测使用的系数（保持 VPC cross-layer 等下游读取 `ps.a, ps.b` 的代码不需要修改）。

#### 1.5.2 观测历史需求

当前 `read_latest_observations(axis, 2)` 只读 2 个样本。扩展为 `read_latest_observations(axis, 4)`。

**前置条件**：`HISTORY_CAP = 1024`（lib.rs:94）已远大于 4，无需扩容。

### 1.6 持久化

#### 1.6.1 learned_bias.bin 版本升级

当前 learned_bias.bin v4 只存 `vocal_efficacy` 等标量。AR-2 系数 `a, b` 不在其中——它们在 predictor 自身的内存中，随 body 重启归零重学。

**设计决策**：Context-Gated 系数**同样不持久化**。

理由：
- AR-2 系数是快速在线学习的（收敛 ~0.5s），重学成本极低
- 持久化 167 × 4 × 4 = 2672 个 f64 增加 ~21KB 到快照，但受益不明显
- 保持和当前 AR-2 的持久化语义一致

如果未来实测发现重启后重学延迟影响发育，可以升级 snapshot 版本加入系数持久化。这是一个**可逆决策**。

### 1.7 发育退火

`ar2_lr_scale(m)` 函数（`lib.rs`）对 Context-Gated 同样适用——`eff_lr` 已包含 maturity 缩放。

额外考虑：新生期（m ≈ 0）某些上下文桶可能长时间没有激活。这不是问题——未激活桶的系数保持初始值 0.0，预测退化为 `prior` only，等同于 AR-2 冷启动。当该上下文首次出现时，高学习率（newborn scale = 3.0）确保快速收敛。

### 1.8 测试清单

| # | 测试 | 断言 |
|---|------|------|
| T1 | 持续上升序列（ctx=3） | 预测 > x₀ |
| T2 | 持续下降序列（ctx=0） | 预测 < x₀ |
| T3 | 反转上升序列（ctx=1） | 预测与 ctx=3 的预测不同（同 x₀,x₁ 不同 ctx） |
| T4 | 4帧不足时退化 | 只有 2 帧 → 退化到 ctx=0 的 2-tap 预测 |
| T5 | 学习只更新激活桶 | 激活 ctx=3 学习后，ctx=0 的系数不变 |
| T6 | maturity 退火兼容 | m=0 时学习率 = base_lr × surprise × 3.0 |
| T7 | 稳态收敛 | 100ms 正弦信号，100 tick 后 MSE < 初始 MSE 的 10% |
| T8 | 外部 prior 正确叠加 | prior=0.5 时，预测增加 0.5 |

---

## 2. 特性 B：模态级群体异常信号

### 2.1 问题陈述

当前预测误差处理（`predictor.rs:84-90`、`viewport.rs:3604-3611`）是**逐轴独立**的：

```
每轴：err = observed - predicted → push_err → surprise EMA → 调制学习率
```

缺失的是**群体级信号**：「在过去 100ms 内，VIB 16 轴中有多少的误差超过了 2σ？如果超过 50%，说明听觉模态整体处于异常状态——不是某个频带出了问题，而是整个声景突然变了。」

这正是 HTM 异常检测的核心：不看单个细胞的预测误差，看**群体中未被预测的比例**。

#### 2.1.1 与 E 组件的关系

前瞻涌现 SDD（`prospection-emergence-scaffold-sdd-2026-06-27.md` §1.1）审计 E 组件（期望自由能/前瞻规划）完成度 15%。群体异常信号是 E 组件的**感知入口**：

- 「世界正常」→ 低异常 → 继续当前策略（反应式预期足够）
- 「世界异常」→ 高异常 → **需要前瞻**（当前模型失效，必须重新评估）

没有这个信号，前瞻涌现缺少**触发条件**。rollout_chain 目前是定时跑的（静默 1.5s 后触发），而不是被「世界变了」触发的。群体异常信号提供了缺失的 bottom-up 触发。

#### 2.1.2 与 convergence_metric.rs 的关系

`convergence_metric.rs`（当前 DEAD CODE）已经计算了系统级 VFE 聚合：

```rust
VFE = 10.0 × mean_beta_variance + 1.0 × prediction_mse + 2.0 × graph_instability
```

其中 `prediction_mse` 组件是全局预测 MSE。群体异常信号可以视为 `prediction_mse` 的**模态分解版**——不是全局一个数，而是每个模态一个数。两者互补：

- `convergence_metric.vfe`：长时间尺度的系统收敛度量（CSV 日志）
- 群体异常信号：短时间尺度的模态级异常检测（30Hz live 路径）

### 2.2 设计

#### 2.2.1 模态分组

复用 `shared_canvas.rs` 已有的模态划分（`shared_canvas.rs:7-18`）：

| 模态 | 轴范围 | 轴数 | 语义 |
|------|--------|------|------|
| VIB | `[0, 16)` | 16 | 听觉频带 |
| VIS | `[16, 144)` | 128 | 4×4 视觉网格 × 8 物理量 |
| INTERO | `[144, 159)` | 15 | 内感受 |
| CODE | `[159, 164)` | 5 | 数字对象 |
| GAUSS_DEF | `[164, 167)` | 3 | 高斯场摘要 + 防御 |

#### 2.2.2 数据结构

**新增文件**：不新增文件。功能添加到 `viewport.rs`。

```rust
pub const ANOMALY_MODALITY_COUNT: usize = 5;

/// Per-modality anomaly state.
pub struct ModalityAnomalyState {
    /// Fraction of axes whose |err| > threshold in the last measurement window.
    pub unpredicted_frac: [f64; ANOMALY_MODALITY_COUNT],
    /// Per-axis running σ (standard deviation of error).
    pub err_sigma: [f64; AXIS_COUNT],
    /// EMA smoothing coefficient for unpredicted_frac.
    pub ema_alpha: f64,
    /// Number of σ above which an axis counts as "unpredicted".
    pub sigma_threshold: f64,
    /// Minimum updates before σ estimate is trusted.
    pub sigma_warmup: u64,
}
```

#### 2.2.3 计算逻辑

**调用位置**：`run_global_refresh_cycle()`，在所有预测器 tick **之后**、发育时钟推进**之前**。

```rust
impl Viewport {
    pub fn tick_modality_anomaly(&self, _now_us: u64) {
        let mut state = self.modality_anomaly.write().unwrap();

        for (m, &(base, size)) in MODALITY_RANGES.iter().enumerate() {
            let mut unpredicted_count = 0usize;
            let mut active_count = 0usize;

            for i in base..(base + size) {
                let ps = self.predictors[i].read().unwrap();
                if ps.total_updates < state.sigma_warmup {
                    continue; // σ estimate not yet reliable
                }
                active_count += 1;

                // Per-axis σ: EMA of |err|
                let err_abs = ps.last_err.abs();
                state.err_sigma[i] = 0.98 * state.err_sigma[i] + 0.02 * err_abs;

                if err_abs > state.sigma_threshold * state.err_sigma[i] {
                    unpredicted_count += 1;
                }
            }

            let raw_frac = if active_count > 0 {
                unpredicted_count as f64 / active_count as f64
            } else {
                0.0
            };

            state.unpredicted_frac[m] = (1.0 - state.ema_alpha) * state.unpredicted_frac[m]
                + state.ema_alpha * raw_frac;
        }
    }
}
```

#### 2.2.4 常量

```rust
const ANOMALY_EMA_ALPHA: f64 = 0.1;      // ~10 tick 半衰期 @ 30Hz ≈ 300ms 窗口
const ANOMALY_SIGMA_THRESHOLD: f64 = 2.0; // 2σ 规则
const ANOMALY_SIGMA_WARMUP: u64 = 30;     // 1 秒 @ 30Hz 才开始信任 σ 估计
```

#### 2.2.5 MODALITY_RANGES 常量

```rust
const MODALITY_RANGES: [(usize, usize); ANOMALY_MODALITY_COUNT] = [
    (0, VIB_BANDS),                                    // VIB: [0, 16)
    (AXIS_VIS_BASE, VIS_AXES),                         // VIS: [16, 144)
    (AXIS_INTERO_BASE, INTERO_AXES),                   // INTERO: [144, 159)
    (AXIS_CODE_BASE, CODE_AXES),                       // CODE: [159, 164)
    (AXIS_GAUSS_BASE, GAUSS_SUMMARY_AXES + DEFENSE_SUMMARY_AXES), // GAUSS+DEF: [164, 167)
];
```

### 2.3 下游消费者

群体异常信号有三个下游接入点，按优先级排列：

#### 2.3.1 P0: 发育时钟调制（最小必要接入）

**接入点**：`advance_maturity_by_progress()` 或其调用者。

```rust
// 在推进 maturity 之前检查
let max_anomaly = state.unpredicted_frac.iter().cloned()
    .fold(0.0_f64, f64::max);

if max_anomaly > ANOMALY_MATURITY_GATE {
    // 世界正在发生异常 → 暂停发育推进
    // 不回退，只暂停。等异常消退后自然恢复推进。
    return;
}
```

**ANOMALY_MATURITY_GATE = 0.4**（40% 的轴未被预测 = 模型严重失效）。

**理由**：在世界剧变期间继续推进发育 maturity 会把噪声写入学习曲线。暂停是安全的——maturity 是单调非递减的，暂停不会回退已达到的能力。

#### 2.3.2 P1: 意图仲裁器触发

**接入点**：`tick_intention_arbitrator()`（`viewport.rs:5893`）。

当前 rollout_chain 在静默 1.5s 后触发（vocal quiet replay）。增加第二个触发条件：

```rust
let anomaly_trigger = self.modality_anomaly.read().unwrap()
    .unpredicted_frac.iter().any(|&f| f > ANOMALY_ROLLOUT_TRIGGER);

if anomaly_trigger || quiet_long_enough {
    // 执行 rollout_chain 前瞻
}
```

**ANOMALY_ROLLOUT_TRIGGER = 0.3**（比发育门控低——先探索，后暂停）。

**理由**：群体异常 = 「世界变了，当前策略可能失效」→ 主动触发前瞻搜索。这是 E 组件的 bottom-up 入口。

#### 2.3.3 P2: L3 FiLM 上下文输入

**接入点**：`context_modulation.rs`（CTX_DIM 已从 8 扩展到 10）。

将 VIB 和 VIS 的 `unpredicted_frac` 输入 FiLM 的上下文向量（需要 CTX_DIM 再 +2 → 12）。

**延迟到 Phase 7**：当前 CTX_DIM=10 已经在 Phase 6 扩展过一次。再扩展需要和 Phase 6 的注意力整合一起考虑。标记为 P2 待定。

### 2.4 接通 convergence_metric.rs

`convergence_metric.rs` 当前是 DEAD CODE。本特性顺带将其接通到 observer 遥测路径：

#### 2.4.1 Live 路径：不接入

VFE 聚合（需要锁定 perturbation_graph 读取全部 relations）太重，不适合 30Hz live 路径。

#### 2.4.2 Observer 路径：接入

在 checkpoint tick（~1Hz）中调用 `vfe_aggregate` 并写 CSV：

**接入点**：body.rs 的 checkpoint 块（当前写 snapshot、vocal_memory 等 sidecar 的地方）。

```rust
// 每个 checkpoint 周期（~1Hz）
let pg = vp.perturbation_graph.read().unwrap();
let avg_mse = vp.modality_anomaly.read().unwrap()
    .unpredicted_frac.iter().sum::<f64>() / ANOMALY_MODALITY_COUNT as f64;
let snap = convergence_metric::vfe_aggregate_at(&pg, avg_mse, tick);
let _ = convergence_metric::append_csv(&convergence_csv_path, &snap);
```

**产出**：`runtime_dir/convergence.csv`，每秒一行，可离线绘制学习曲线。

### 2.5 测试清单

| # | 测试 | 断言 |
|---|------|------|
| T1 | 所有轴 err=0 | 所有模态 unpredicted_frac = 0.0 |
| T2 | VIB 全部 16 轴 err > 3σ | VIB unpredicted_frac > 0.9；其他模态不受影响 |
| T3 | warmup 期间 | total_updates < 30 的轴不计入分母 |
| T4 | EMA 衰减 | 异常脉冲后 ~10 tick，frac 衰减到 < 0.1 |
| T5 | 发育门控 | unpredicted_frac > 0.4 时 maturity 不推进 |
| T6 | 发育门控恢复 | anomaly 消退后 maturity 恢复推进 |
| T7 | rollout 触发 | unpredicted_frac > 0.3 时触发 rollout_chain |
| T8 | convergence CSV | checkpoint 后 CSV 文件存在且格式正确 |
| T9 | 模态隔离 | 视觉异常不污染听觉的 unpredicted_frac |

---

## 3. 实施计划

### 3.1 阶段划分

| 阶段 | 内容 | 估算 | 前置 |
|------|------|------|------|
| **B-1** | viewport.rs 加 ModalityAnomalyState + tick_modality_anomaly | ~80 行 | 无 |
| **B-2** | body.rs 接入 run_global_refresh_cycle + 发育门控 | ~20 行 | B-1 |
| **B-3** | 接通 convergence_metric.rs 到 checkpoint | ~15 行 | B-1 |
| **B-4** | 意图仲裁器异常触发 | ~10 行 | B-1 |
| **B-T** | 9 个测试 | ~200 行 | B-1..B-4 |
| **A-1** | predictor.rs 重写 Ar2 → ContextGated | ~100 行 | 无 |
| **A-2** | PredictorState 扩展 + viewport 接口 | ~30 行 | A-1 |
| **A-3** | body.rs 读取 4 帧 + 传递 ctx | ~10 行 | A-1, A-2 |
| **A-T** | 8 个测试 | ~150 行 | A-1..A-3 |

### 3.2 风险

| 风险 | 概率 | 缓解 |
|------|------|------|
| Context-Gated 收敛变慢（每桶样本 ÷4） | 中 | newborn scale=3.0 补偿；如果不够可增大 base_lr |
| 群体异常过度触发（正常探索也触发） | 低 | EMA 平滑 + sigma_warmup 30 tick 避免冷启动误报 |
| VIS 128 轴中少量异常被 128 分母稀释 | 中 | 可后续拆分 VIS 为 16 个子区域（每个 8 轴），但初始实现先按整个模态 |
| convergence CSV 磁盘增长 | 低 | 1Hz × ~60 bytes/行 ≈ 5MB/天，可接受 |

### 3.3 不做的事

| 提议 | 不做原因 |
|------|---------|
| HTM 二值 SDR | 连续稀疏码信息量更高（§0 审计结论） |
| HTM 永久性阈值 | 连续权重 + surprise 调制已等效（§0 审计结论） |
| 千脑多柱分拆 | 拆统一基底代价大于收益（§0 审计结论） |
| N_CTX=8 | 样本稀释风险，4 桶覆盖核心区分（§1.3.3） |
| 系数持久化 | 收敛快（~0.5s），重学成本低（§1.6.1） |
| L3 FiLM 异常输入 | CTX_DIM 刚扩展过，延迟到 Phase 7（§2.3.3） |

---

## 4. 公理一致性检查

| 公理 | Context-Gated AR-4 | 群体异常信号 | 合规 |
|------|-------------------|-------------|------|
| ①数值不落地 | 系数是 f64，无符号标签 | frac 是 f64 标量 | ✓ |
| ②路线 A 纯涌现 | 上下文桶由观测轨迹自动选择，不手写规则 | 异常阈值是统计量（2σ），不手写模式 | ✓ |
| ③局部学习 | 只更新激活桶的系数 | 只读 predictor state，不修改学习规则 | ✓ |
| ⑤粗到细 | newborn 高 lr 快速收敛，mature 低 lr 精细调整 | warmup 期不计入 → 新生期不触发异常 | ✓ |
| ⑧无语义层 | 无符号/标签/分类器 | 模态划分是物理分组不是语义分类 | ✓ |
