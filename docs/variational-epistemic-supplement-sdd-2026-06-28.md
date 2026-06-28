# 变分认识论补充件 — 软件设计文档 (SDD)

**日期**：2026-06-28
**版本**：v1
**状态**：SPEC（待实施）
**主文档**：`cognitive-middle-layers-sdd-2026-06-27.md`
**兄弟文档**：`temporal-credit-horizon-supplement-2026-06-28.md`（增补 A/B）
**前置阅读**：主 SDD §1-§13、`prospection-emergence-scaffold-sdd-2026-06-27.md`

---

## 0. 文档定位

这是主 SDD 和时间信用补充件的**第二独立补充件**，不修改任何现有文档内容。

主 SDD 解决了**空间维度的特征提取与模式匹配**（L0-L4 五层认知管线）。
时间信用补充件（增补 A/B）解决了**时间维度的信用分配**（级联 elig trace + 三因子 + 压缩重放）。
本文档填补第三个维度：**不确定性感知与认识价值驱动的行动选择**。

**一句话**：当前系统所有推断和决策基于点估计（标量 strength），没有不确定性量化→无法计算信息增益→无法回答"看哪里能学到最多？"；三个增补（C1 Beta 分布 + C2 认识价值 rollout + C3 正向重放 EFE）以最小侵入方式让系统获得不确定性感知能力，使前瞻从局部信息中涌现。

---

## 1. 问题陈述

### 1.1 论点 4 原始批评

> "项目没有全局目标函数——既拒绝了反向传播，又没有替代的变分界。没有 ELBO、没有 KL 散度、没有自由能最小化的显式目标。rollout_chain 是贪心搜索，不是期望自由能最小化。"

### 1.2 两份 SDD 后的修正评估

| 子论断 | 原始判定 | 修正判定 | 依据 |
|--------|---------|---------|------|
| 无信息论保证 | 成立 | **部分推翻** | L2 稀疏编码 `min ‖y-Da‖² + λ‖a‖₁` = Laplace 先验下 MAP/ELBO 特例 (Olshausen 1996, Seeger 2008) |
| 无 ELBO / KL | 成立 | **部分推翻** | L2 的 `λ‖a‖₁` 在概率解释下是 KL(posterior ‖ Laplace prior)；L1 精度加权 = 精确贝叶斯多感官整合 |
| rollout 贪心 | 成立 | **仍然成立** | rollout_chain 依然是 argmax，无认识价值项 |
| 无数学基础实现前瞻 | 成立 | **推翻** | 三因子规则 = BPTT 局部近似 (Bellec 2020)；L4→L3 梯度反传已在跑 (cognitive_pipeline.rs:79) |
| 只有反应式预期 | 成立 | **部分推翻** | ④+②+⑤ 涌现闭环 + 正向重放已存在 |

### 1.3 修正后精确定位的三个缺口

| # | 缺口 | 现状 | 后果 |
|---|------|------|------|
| G1 | **关系图无不确定性表示** | `Relation.strength` 是 f64 标量 | 无法区分"看了1000次的0.5"和"看了2次的0.5" |
| G2 | **行动选择无认识价值** | `score = strength × expressivity` 纯实用 | 不会主动探索不确定路径来学习 |
| G3 | **正向重放不追踪预期不确定性** | 排序仅用 surprise + recency | 不能估计"走这条路能学到多少" |

### 1.4 设计原则（继承业主决策）

- ❌ 不引入手写 EFE 规划器、MCTS、tree search
- ❌ 不引入全局损失函数或外部奖励
- ✅ 最小侵入：在现有结构上添加最少的字段和计算
- ✅ 向后兼容：所有增补在"退化"配置下行为等价于旧系统
- ✅ 独立可测试：C1/C2/C3 各有独立价值，可单独上线

---

## 2. 已有数学基础盘点（本补充件的"地基"）

在设计新增补之前，明确系统**已有**的信息论装置——本补充件是在这些之上叠加，非从零开始。

| 组件 | 数学性质 | 文件 | 参考 |
|------|---------|------|------|
| L2 稀疏编码 | MAP 推断 / ELBO 特例 (Laplace 先验) | feature_extraction.rs:72-104 | Olshausen 1996; Seeger 2008 |
| L1 精度融合 | 精确贝叶斯多感官整合 | shared_canvas.rs:95-134 | Ernst & Banks 2002 |
| L4 Hopfield 检索 | 能量函数最小化 | pattern_memory.rs:161-179 | Ramsauer 2020; Hopfield 2024 Nobel |
| L4→L3 梯度反传 | 链式法则梯度下降 | context_modulation.rs:65-109 | Millidge 2021 |
| 三因子学习规则 | BPTT 的 rank-1 局部近似 | perturbation_graph.rs:187-203 | Bellec 2020 (e-prop) |
| 压缩反向重放 | 时间维度反向传播等价物 | replay_engine.rs:165-185 | Foster & Wilson 2006 |

**关键认知**：系统有**分布式局部变分界**，缺的不是"目标函数"而是**不确定性量化**使这些局部界能汇聚为有方向的探索。

---

## 3. 增补 C1：Beta 分布关系强度

### 3.1 科学依据

**Thompson 1933**, *Biometrika*: "On the Likelihood that One Unknown Probability Exceeds Another"
- 后验采样 = 最优探索/利用权衡的贝叶斯解
- 对未知概率维护 Beta(α, β) 后验，采样决策 → 不确定选项自动获得更多探索

**Russo, Van Roy, Kazerouni, Osband & Wen 2018**, *Foundations and Trends in ML*: "A Tutorial on Thompson Sampling"
- Thompson Sampling 的 Bayesian regret = O(√(KT log T)) — 接近最优
- 不需要调 ε-greedy 参数、不需要 UCB 置信界 — 后验采样自动权衡

**Gershman 2018**, *Cognition*: "Deconstructing the human algorithms for exploration"
- 人类探索行为与 Thompson Sampling 惊人吻合
- 多巴胺信号的随机性不是噪声——是**采样过程的计算功能**

**Daw, O'Doherty, Dayan, Seymour & Dolan 2006**, *Nature*: "Cortical substrates for exploratory decisions in humans"
- fMRI 证据：人类在不确定时切换到探索模式
- 切换触发 = 模型的**相对不确定性** → 需要量化不确定性才能触发

### 3.2 数学规格

#### 3.2.1 Relation 结构扩展

替换标量 `strength: f64` 为 Beta 分布参数对：

```
新增字段:
  alpha: f64     // 伪计数"命中" — 初始 1.0
  beta_param: f64 // 伪计数"未中" — 初始 1.0（避免与 Rust 关键字冲突）

派生量（不存储，实时算）:
  mean()      = α / (α + β)                        // 等价于旧 strength
  variance()  = αβ / ((α+β)² × (α+β+1))            // 不确定性量化
  uncertainty() = 1.0 / (α + β)                     // 简化不确定性度量
  sample(u)   = beta_icdf(u, α, β)                  // Thompson 采样
```

#### 3.2.2 信用更新规则

**旧规则**（post_hoc_check_modulated, perturbation_graph.rs:2128-2133）：

```
if correct:  strength = (strength + delta * 0.01).min(1.0)
if !correct: strength = (strength - delta * 0.05).max(0.0)
```

**新规则**：

```
let delta = elig × modulation;

if correct:
    alpha += delta × CREDIT_HIT_SCALE          // 累积命中证据
if !correct:
    beta_param += delta × CREDIT_MISS_SCALE    // 累积未中证据

// strength 从 mean() 派生，不再独立更新
strength = alpha / (alpha + beta_param)

参数:
  CREDIT_HIT_SCALE  = 1.0   // 命中加 1 伪观察
  CREDIT_MISS_SCALE = 1.0   // 未中加 1 伪观察
```

**不对称性说明**：旧规则有不对称（+0.01 vs ×0.95）。新规则用对称的贝叶斯更新——不对称性通过三因子调制 M(t) 的方向自然产生：M(t) > 1.0 时正调制放大 hit，M(t) < 1.0 时弱调制放大 miss。不需要硬编码不对称。

#### 3.2.3 衰减（防无限积累）

Beta 参数无限增长 → variance → 0 → 系统过度自信、拒绝新证据。生物上不合理（突触有可塑性上限）。

```
每 BETA_DECAY_INTERVAL ticks (1000 ≈ 33s @30Hz):
  alpha     = max(alpha × BETA_DECAY_RATE, BETA_FLOOR)
  beta_param = max(beta_param × BETA_DECAY_RATE, BETA_FLOOR)

参数:
  BETA_DECAY_RATE     = 0.995    // 每 33s 衰减 0.5%
  BETA_DECAY_INTERVAL = 1000     // ticks
  BETA_FLOOR          = 1.0      // 最低伪计数（= 均匀先验）
```

**效果**：旧证据缓慢衰减，保持系统对新信息的敏感度。半衰期 ≈ `1000 × ln(2) / (1-0.995) ≈ 139,000 ticks ≈ 77 min`——足够长期关系保持信用，足够短陈旧关系重开不确定性。

#### 3.2.4 初始化

```
新建 Relation:
  alpha      = 1.0    // 均匀先验 = 最大不确定性
  beta_param = 1.0
  strength   = 0.5    // = mean() = 1/(1+1)

observe_relation 更新:
  // 旧: strength = (strength + α*(1-strength)).clamp(0,1)
  // 新: alpha += OBSERVE_HIT (0.5)  — 观察到 = 弱命中证据
  //     strength = mean()
```

#### 3.2.5 与三因子规则的交互

增补 A 的 `Δstrength = elig × M(t)` 改为证据累积：

```
if M(t) > 0.0:
    alpha     += elig × M(t) × CREDIT_HIT_SCALE    // 正调制 = 命中
if M(t) < 0.0:
    beta_param += elig × |M(t)| × CREDIT_MISS_SCALE // 负调制 = 未中

strength = mean()
```

中性调制 (M(t) ≈ 1.0) 时退化到纯 hit 证据累积（向后兼容）。

### 3.3 向后兼容

- `strength` 字段保留——从 `mean()` 派生更新
- 所有读 `strength` 的下游代码**零改动**（值相同，只是来源变了）
- `alpha` 和 `beta_param` 不参与序列化 v1——首次加载时从 `strength` 逆推初始化：
  ```
  let total = BETA_INIT_TOTAL;          // 10.0 — 默认"已看过几次"
  alpha      = strength × total;
  beta_param = (1.0 - strength) × total;
  ```

### 3.4 Rust 结构变更

```rust
// perturbation_graph.rs — Relation 结构扩展

// ── Beta distribution constants (C1) ──────────────────────────────────
pub const PG_BETA_FLOOR: f64 = 1.0;
pub const PG_BETA_DECAY_RATE: f64 = 0.995;
pub const PG_BETA_DECAY_INTERVAL: u32 = 1000;
pub const PG_CREDIT_HIT_SCALE: f64 = 1.0;
pub const PG_CREDIT_MISS_SCALE: f64 = 1.0;
pub const PG_BETA_INIT_TOTAL: f64 = 10.0;
pub const PG_OBSERVE_HIT: f64 = 0.5;

pub struct Relation {
    // ... 所有现有字段不变 ...

    /// Beta posterior α — "hit" pseudo-count (Thompson 1933).
    pub alpha: f64,
    /// Beta posterior β — "miss" pseudo-count. Named beta_param to avoid
    /// collision with Rust keyword.
    pub beta_param: f64,
}

impl Relation {
    /// Posterior mean = α/(α+β). Replaces direct strength assignment.
    pub fn mean(&self) -> f64 {
        self.alpha / (self.alpha + self.beta_param)
    }

    /// Uncertainty = 1/(α+β). High when few observations, low when many.
    pub fn uncertainty(&self) -> f64 {
        1.0 / (self.alpha + self.beta_param)
    }

    /// Posterior variance of the Beta distribution.
    pub fn variance(&self) -> f64 {
        let s = self.alpha + self.beta_param;
        (self.alpha * self.beta_param) / (s * s * (s + 1.0))
    }

    /// Decay both pseudo-counts toward the floor (prevents over-confidence).
    pub fn decay_beta(&mut self) {
        self.alpha = (self.alpha * PG_BETA_DECAY_RATE).max(PG_BETA_FLOOR);
        self.beta_param = (self.beta_param * PG_BETA_DECAY_RATE).max(PG_BETA_FLOOR);
        self.strength = self.mean();
    }

    /// Record a credit observation via Beta update.
    pub fn credit_hit(&mut self, amount: f64) {
        self.alpha += amount * PG_CREDIT_HIT_SCALE;
        self.strength = self.mean();
    }

    pub fn credit_miss(&mut self, amount: f64) {
        self.beta_param += amount * PG_CREDIT_MISS_SCALE;
        self.strength = self.mean();
    }
}
```

### 3.5 接入点

```
文件: perturbation_graph.rs
修改: Relation 结构 + post_hoc_check_modulated + observe_relation
新增: mean()/uncertainty()/variance()/decay_beta()/credit_hit()/credit_miss()
调用: 每 PG_BETA_DECAY_INTERVAL ticks 批量调 decay_beta()
序列化: alpha/beta_param 加入 sidecar bin 格式 (版本号 bump)
```

### 3.6 验证标准

| 测试 | 通过标准 |
|------|---------|
| `beta_mean_equals_strength` | 新建 Relation, `mean() == strength == 0.5` |
| `beta_credit_hit_raises_mean` | 10 次 credit_hit(1.0) 后 mean() > 0.8 |
| `beta_credit_miss_lowers_mean` | 10 次 credit_miss(1.0) 后 mean() < 0.2 |
| `beta_uncertainty_decreases_with_evidence` | 100 次更新后 uncertainty() < 新建时的 1/50 |
| `beta_decay_prevents_overconfidence` | 10000 次 decay → α,β ≈ BETA_FLOOR |
| `beta_init_from_legacy_strength` | strength=0.7 → α=7.0, β=3.0, mean()≈0.7 |
| `post_hoc_modulated_uses_beta` | post_hoc_check 后 α 或 β 变化，strength = mean() |

---

## 4. 增补 C2：认识价值驱动的 rollout

### 4.1 科学依据

**Friston, Rigoli, Ognibene, Mathys, Fitzgerald & Pezzulo 2015**, *Cognitive Neuroscience*: "Active inference and epistemic value"
- 期望自由能 = **实用价值** + **认识价值**
- 实用价值 = 到达偏好状态的程度
- 认识价值 = 行动能消除多少不确定性
- 公式: `G(π) = E_q[D_KL(P(o|s) ‖ P(o))] + E_q[log P(o)]`

**Schwartenbeck, FitzGerald, Mathys, Dolan & Friston 2015**, *Cerebral Cortex*: "The Dopaminergic Midbrain Encodes the Expected Certainty about Desired Outcomes"
- 多巴胺不仅编码奖励预测误差——也编码**期望确定性**
- 三因子规则的"多巴胺通道"生物上已包含认识价值

**Gottlieb, Oudeyer, Lopes & Baranes 2013**, *Trends in Cognitive Sciences*: "Information-seeking, curiosity, and attention"
- 好奇心 = 认识价值的计算实现
- 婴儿习惯化-去习惯化 = 认识价值驱动的注意力转移

**Schmidhuber 2010**, *IEEE Trans. Autonomous Mental Development*: "Formal Theory of Creativity, Fun, and Intrinsic Motivation"
- 好奇心 = 学习进度最大化（与认识价值数学等价）
- 已在 INTERO_LEARNING_PROGRESS 轴部分体现

### 4.2 数学规格

#### 4.2.1 复合评分

**旧评分**（perturbation_graph.rs:1649-1650, 1669, 1704）：

```
score(R) = R.strength × R.expressivity
```

**新评分**：

```
pragmatic(R) = R.mean() × R.expressivity        // 实用价值（不变）
epistemic(R) = R.uncertainty()                   // 认识价值 = 1/(α+β)
score(R)     = pragmatic(R) + w_epi × epistemic(R)

其中:
  w_epi = epistemic_weight(maturity)
        = anticipation_babble_floor(maturity) × W_EPI_MAX

参数:
  W_EPI_MAX = 2.0       // 认识价值最大权重
```

#### 4.2.2 认识价值权重退火

```
epistemic_weight(maturity):
  maturity ≈ 0.0  →  0.30 × 2.0 = 0.60  (高探索：年幼体追新奇)
  maturity ≈ 0.5  →  0.175 × 2.0 = 0.35  (中间态)
  maturity ≈ 1.0  →  0.05 × 2.0 = 0.10  (低探索：成熟体利用已知)
```

使用 `anticipation_babble_floor(maturity)` 作为退火基函数——复用前瞻涌现脚手架已设计的退火曲线，不引入新参数。

#### 4.2.3 与 babble floor 的关系

前瞻涌现脚手架（§6.2.3）的 babble floor 让年幼体的弱 rollout 也能进入意图：

```
effective_score = score.max(anticipation_babble_floor(maturity))
```

C2 改为：

```
effective_score = (pragmatic + w_epi × epistemic).max(anticipation_babble_floor(maturity))
```

babble floor 仍作为**绝对地板**（防 0×x=0 死锁），认识价值在地板之上提供**有方向的探索偏好**。两者互补：
- babble floor = 无方向噪声探索（"随便走走"）
- 认识价值 = 有方向信息探索（"走最不确定的路"）

### 4.3 改造点

#### 4.3.1 strongest_out_relation

```rust
// perturbation_graph.rs:1647-1653
// 旧:
pub fn strongest_out_relation(&self, from: Endpoint) -> Option<&Relation> {
    self.out_relations(from).max_by(|a, b| {
        let av = a.strength * a.expressivity;
        let bv = b.strength * b.expressivity;
        av.partial_cmp(&bv).unwrap()
    })
}

// 新: 接受 epistemic 权重参数
pub fn strongest_out_relation_epi(
    &self,
    from: Endpoint,
    w_epi: f64,
) -> Option<&Relation> {
    self.out_relations(from).max_by(|a, b| {
        let av = a.mean() * a.expressivity + w_epi * a.uncertainty();
        let bv = b.mean() * b.expressivity + w_epi * b.uncertainty();
        av.partial_cmp(&bv).unwrap()
    })
}

// 旧 API 保留（w_epi=0 退化）:
pub fn strongest_out_relation(&self, from: Endpoint) -> Option<&Relation> {
    self.strongest_out_relation_epi(from, 0.0)
}
```

#### 4.3.2 rollout_chain

```rust
// perturbation_graph.rs:1658-1680
// 新: rollout_chain 接受 w_epi
pub fn rollout_chain_epi(
    &self,
    from: Endpoint,
    k_steps: usize,
    w_epi: f64,
) -> Vec<(Endpoint, RelId, f64)> {
    let mut chain = Vec::with_capacity(k_steps);
    let mut cursor = from;
    let mut visited = std::collections::HashSet::new();
    visited.insert(from);
    for _ in 0..k_steps {
        if let Some(rel) = self.strongest_out_relation_epi(cursor, w_epi) {
            let score = rel.mean() * rel.expressivity
                      + w_epi * rel.uncertainty();
            if !visited.insert(rel.to) {
                break;
            }
            chain.push((rel.to, rel.id, score));
            cursor = rel.to;
        } else {
            break;
        }
    }
    chain
}

// 旧 API 保留:
pub fn rollout_chain(&self, from: Endpoint, k_steps: usize) -> ... {
    self.rollout_chain_epi(from, k_steps, 0.0)
}
```

#### 4.3.3 sample_out_relation

```rust
// perturbation_graph.rs:1690-1731
// score lambda 改为:
let score = |r: &Relation| {
    (r.mean() * r.expressivity + w_epi * r.uncertainty()).max(0.0)
};
```

#### 4.3.4 viewport 意图仲裁器

```rust
// viewport.rs:5792-5835
// 新增: 从 maturity 计算 w_epi
let w_epi = crate::anticipation_babble_floor(maturity) * W_EPI_MAX;

// 感知候选:
if let Some(r) = graph.strongest_out_relation_epi(ep, w_epi) {
    let score = r.mean() * r.expressivity + w_epi * r.uncertainty();
    ...
}

// Rollout:
graph.rollout_chain_epi(ep, ROLLOUT_HORIZON, w_epi)
```

### 4.4 Rust 结构变更

```rust
// lib.rs
pub const W_EPI_MAX: f64 = 2.0;

pub fn epistemic_weight(maturity: f64) -> f64 {
    anticipation_babble_floor(maturity) * W_EPI_MAX
}
```

### 4.5 涌现效果预测

| 场景 | 旧行为 | 新行为 |
|------|--------|--------|
| 新关系(α=1,β=1) vs 旧强关系(α=500,β=50) | 选旧（0.91 >> 0.5） | 新关系得到 epistemic 加权 0.5 × w_epi，有机会被探索 |
| 两条同 strength=0.5 的关系，一条看过 2 次一条看过 1000 次 | 随机（分数相同） | 选看过 2 次的（uncertainty 0.5 >> 0.001） |
| 成熟体面对已知环境 | 纯利用 | w_epi ≈ 0.10，几乎纯利用（正确） |
| 年幼体面对新环境 | babble floor 随机探索 | w_epi ≈ 0.60，**有方向地**探索高不确定性路径 |

### 4.6 验证标准

| 测试 | 通过标准 |
|------|---------|
| `epistemic_weight_decreases_with_maturity` | mat=0.0 时 w > mat=1.0 时 w |
| `strongest_epi_prefers_uncertain_at_equal_strength` | 两条 mean=0.5 关系，α+β 小的被选中 |
| `strongest_epi_reverts_to_pragmatic_when_w0` | w_epi=0 时结果与旧 API 相同 |
| `rollout_epi_explores_uncertain_branch` | Y 分叉图：强但确定 vs 弱但不确定，w_epi>0 时选不确定分支 |

---

## 5. 增补 C3：正向重放的预期信息增益

### 5.1 科学依据

**Mattar & Daw 2018**, *Nature Neuroscience*: "Prioritized memory access explains planning and hippocampal replay"
- 优先级 = **gain** × **need**
- gain = 信息传播价值（高 TD 误差的经历）
- need = 决策相关性（即将面对的选择）
- 本设计用 C1 的 uncertainty 直接计算 gain

**Stachenfeld, Botvinick & Gershman 2017**, *Nature Neuroscience*: "The hippocampus as a predictive map"
- 海马编码继任表征 (SR) = "从这里出发未来会到哪些状态"
- 转移矩阵（增补 B 的 `PatternTransition`）= SR 的经验近似
- 价值 = SR × 奖励权重 → 本设计中价值 = SR × 不确定性减少

**Millidge, Tschantz & Buckley 2020**, *Neural Computation*: "Whence the Expected Free Energy?"
- 证明 EFE 可沿**采样轨迹**估计，不需要全状态空间枚举
- 正向重放轨迹 + Beta 不确定性 = 蒙特卡洛 EFE 近似

**Diba & Buzsáki 2007**, *Nature Neuroscience*
- 正向重放(841/887个事件)集中在行动**前** = 前瞻规划

### 5.2 数学规格

#### 5.2.1 正向重放优先级增强

**旧优先级**（replay_engine.rs:135-146）：

```
priority(step) = W_SURPRISE × (1 - predictability)
               + W_SOCIAL × social
               + W_RECENCY × recency
```

**新优先级**（C3 增强）：

```
priority(step) = W_SURPRISE × (1 - predictability)
               + W_SOCIAL × social
               + W_RECENCY × recency
               + W_INFO_GAIN × info_gain(step)    // 新增项
```

#### 5.2.2 信息增益计算

对正向重放轨迹中的每个 pattern step，通过关联的 PG relations 估计信息增益：

```
info_gain(pattern_id) =
    Σ_R∈related_relations(pattern_id) [
        uncertainty_before(R) - uncertainty_after(R)
    ]

其中:
  uncertainty_before(R) = 1.0 / (R.alpha + R.beta_param)
  uncertainty_after(R)  = 1.0 / (R.alpha + R.beta_param + 1.0)

简化:
  info_gain(pattern_id) =
    Σ_R [ 1/(α+β) - 1/(α+β+1) ]
  = Σ_R [ 1 / ((α+β)(α+β+1)) ]
```

**性质**：
- 新关系（α=1, β=1）: 每条贡献 1/(2×3) = 0.167
- 旧关系（α=50, β=10）: 每条贡献 1/(60×61) = 0.00027
- 信息增益自然偏向不确定的关系

#### 5.2.3 关联关系查找

从 pattern_id 到 PG relations 的映射需要一个接口。最小设计：

```
info_gain_for_pattern: &dyn Fn(usize) -> f64
```

回调由 `CognitivePipeline` 提供，内部通过 L4 pattern 的特征向量与 PG 节点 sig 做最近邻匹配，找到相关 relations，计算信息增益。

简化 Phase 1 实现：如果无法从 pattern_id 映射到 PG relations，退化为 `info_gain = 0.0`（不影响现有行为）。Phase 2 接入 L4↔PG 交互（主 SDD Phase 5-6 工作范围）后信息增益生效。

### 5.3 Rust 结构变更

```rust
// replay_engine.rs

pub const PRIORITY_W_INFO_GAIN: f64 = 1.5;

impl ReplayEngine {
    pub fn generate_replay_epi(
        &self,
        trigger: ReplayTrigger,
        predictability: &dyn Fn(usize) -> f64,
        info_gain: &dyn Fn(usize) -> f64,     // C3 新增回调
        now_us: u64,
    ) -> Vec<ReplayStep> {
        // ... 现有逻辑 ...

        let mut ranked: Vec<(usize, f64)> = chain.iter().map(|&pid| {
            let surprise_score = 1.0 - predictability(pid);
            let age_us = /* 现有计算 */;
            let recency = (-age_us / PRIORITY_TAU_RECENCY_US).exp();
            let ig = if direction == ReplayDirection::Forward {
                info_gain(pid)  // 仅正向重放使用信息增益
            } else {
                0.0             // 反向重放不加信息增益（它的任务是信用分配不是探索）
            };
            PRIORITY_W_SURPRISE * surprise_score
                + PRIORITY_W_SOCIAL * 0.0
                + PRIORITY_W_RECENCY * recency
                + PRIORITY_W_INFO_GAIN * ig
        }).enumerate().map(|(i, score)| (chain[i], score)).collect();

        // ... 后续压缩和排序不变 ...
    }
}
```

旧 `generate_replay` 保留，内部调 `generate_replay_epi` 传 `info_gain = |_| 0.0`。

### 5.4 与 CognitivePipeline 的集成

```rust
// cognitive_pipeline.rs — trigger_replay 增强

pub fn trigger_replay_epi(
    &mut self,
    trigger: ReplayTrigger,
    pg_info_gain: &dyn Fn(usize) -> f64,  // 由 viewport 提供
    axes: &[f64; AXIS_COUNT],
    maturity: f64,
    t_us: u64,
) -> Vec<ReplayStep> {
    // ... 现有逻辑，改调 generate_replay_epi ...
}
```

### 5.5 涌现效果预测

```
静息态触发正向重放:
  1. ReplayEngine 生成正向轨迹（已有）
  2. 每步计算 info_gain（C3 新增）
  3. 高不确定性步骤获得高优先级
  4. 这些步骤优先被重放
  5. 重放注入 L2/L3/L4 管线（已有）
  6. L4 匹配误差 → L3 权重更新（已有）
  7. 相关 PG relations 的 elig 重激活（增补 A）
  8. 三因子规则门控学习（增补 A）

净效果: 系统在静息态优先"想象"不确定的未来
       → 涌现式期望自由能最小化
       → 不是手写 EFE 规划器，是重放优先级自然偏向高信息增益
```

### 5.6 验证标准

| 测试 | 通过标准 |
|------|---------|
| `info_gain_higher_for_uncertain_relations` | α=1,β=1 的信息增益 > α=50,β=10 的 |
| `forward_replay_ranks_by_info_gain` | 两条正向轨迹，高 IG 的排在前 |
| `reverse_replay_ignores_info_gain` | 反向重放的排序不受 info_gain 影响 |
| `zero_info_gain_matches_old_behavior` | info_gain=0 时输出与旧 generate_replay 相同 |

---

## 6. 三个增补的联合数据流

```
正常 tick:
  L0 → L1 → L2 → L3 → L4.query()
                         │
                         ├── Completed → post_hoc_check_modulated
                         │     旧: strength ± delta
                         │     新: alpha/beta_param ± delta [C1]
                         │         strength = mean()
                         │
                         └── PG relations 评分
                               旧: strength × expressivity
                               新: mean() × expressivity + w_epi × uncertainty() [C2]

意图仲裁器:
  strongest_out_relation:
    旧: argmax(strength × expressivity)
    新: argmax(mean() × expressivity + w_epi × uncertainty()) [C2]

  rollout_chain:
    旧: K步 argmax(strength × expressivity)
    新: K步 argmax(mean() × expressivity + w_epi × uncertainty()) [C2]

正向重放 (Quiet 触发):
  旧优先级: W_surprise × surprise + W_recency × recency
  新优先级: + W_info_gain × Σ[1/((α+β)(α+β+1))] [C3]

衰减 (每 1000 ticks):
  旧: 无
  新: alpha *= 0.995, beta_param *= 0.995 [C1]
```

---

## 7. 信息论保证总结

实施 C1+C2+C3 后系统拥有的信息论装置：

| 装置 | 类型 | 来源 |
|------|------|------|
| L2 `‖y-Da‖² + λ‖a‖₁` | MAP/ELBO (Laplace 先验) | 主 SDD, 已有 |
| L1 精度加权融合 | 贝叶斯最优多感官整合 | 主 SDD, 已有 |
| L4 Hopfield softmax | 能量函数最小化 | 主 SDD, 已有 |
| L4→L3 梯度反传 | 链式法则梯度下降 | 主 SDD, 已有 |
| 三因子学习规则 | BPTT rank-1 近似 | 增补 A, 已有 |
| **Beta 后验更新** | **贝叶斯后验推断** | **C1, 本文档** |
| **认识价值评分** | **期望信息增益** | **C2, 本文档** |
| **正向重放 EFE** | **蒙特卡洛 EFE 近似** | **C3, 本文档** |

**对论点 4 的完整回应**：系统没有单一全局 ELBO——它有**分布式的局部变分装置**（L2 ELBO + L1 贝叶斯 + L4 能量 + Beta 后验），通过**数学上有保证的近似**（e-prop ≈ BPTT, PC ≈ BP）在层间传递梯度，并通过**认识价值 + 正向重放**实现涌现式期望自由能最小化。这不是"没有目标函数"——是"多个局部目标函数通过信用链路和重放机制协调"。

---

## 8. 实施计划

### 8.1 依赖顺序

```
C1 Beta 分布 ──→ C2 认识价值 rollout ──→ C3 正向重放 EFE
(基础)           (应用)                  (整合)

C1 与增补 A (T1-T4) 可并行：
  增补 A 修改 elig/三因子 (不触碰 strength 表示)
  C1 修改 strength 表示 (不触碰 elig/三因子)
  两者在 post_hoc_check_modulated 汇合

C3 需要增补 B (T5-T6) 的 ReplayEngine 已存在
```

### 8.2 分阶段

| 阶段 | 内容 | 文件 | 前置 | 估计复杂度 |
|------|------|------|------|-----------|
| **C1** | Beta 分布替换标量 strength | perturbation_graph.rs | 无 | 中 |
| **C2** | 认识价值加入评分 + 新 API | perturbation_graph.rs, viewport.rs, lib.rs | C1 | 低 |
| **C3** | 正向重放信息增益优先级 | replay_engine.rs, cognitive_pipeline.rs | C1 | 低 |

### 8.3 每阶段变更清单

#### C1 变更清单

| 文件 | 变更 | 描述 |
|------|------|------|
| perturbation_graph.rs | 常量 | 6 个 PG_BETA_* 常量 |
| perturbation_graph.rs | Relation 结构 | +alpha, +beta_param 字段 |
| perturbation_graph.rs | impl Relation | +mean(), +uncertainty(), +variance(), +decay_beta(), +credit_hit(), +credit_miss() |
| perturbation_graph.rs | post_hoc_check_modulated | 替换 strength ± → credit_hit/credit_miss |
| perturbation_graph.rs | observe_relation | 替换 strength EMA → alpha += OBSERVE_HIT |
| perturbation_graph.rs | Relation 初始化 | 所有构造点加 alpha=1.0, beta_param=1.0 |
| perturbation_graph.rs | 序列化/反序列化 | alpha/beta_param 加入 bin 格式，版本号 bump |
| perturbation_graph.rs | 测试 | 7 个新测试 |

#### C2 变更清单

| 文件 | 变更 | 描述 |
|------|------|------|
| lib.rs | 常量+函数 | W_EPI_MAX, epistemic_weight(maturity) |
| perturbation_graph.rs | 方法 | +strongest_out_relation_epi(from, w_epi) |
| perturbation_graph.rs | 方法 | +rollout_chain_epi(from, k, w_epi) |
| perturbation_graph.rs | 方法 | sample_out_relation 内部 score 改用复合评分 |
| viewport.rs | 意图仲裁器 | 计算 w_epi，调用 _epi 变体 |
| perturbation_graph.rs | 测试 | 4 个新测试 |

#### C3 变更清单

| 文件 | 变更 | 描述 |
|------|------|------|
| replay_engine.rs | 常量 | PRIORITY_W_INFO_GAIN |
| replay_engine.rs | 方法 | +generate_replay_epi(trigger, predictability, info_gain, now_us) |
| replay_engine.rs | 旧 API | generate_replay 内部调 _epi 传 zero info_gain |
| cognitive_pipeline.rs | 方法 | +trigger_replay_epi |
| replay_engine.rs | 测试 | 4 个新测试 |

---

## 9. 参数总表

| 参数 | 值 | 增补 | 理由 |
|------|-----|------|------|
| `PG_BETA_FLOOR` | 1.0 | C1 | 最低伪计数 = 均匀先验 |
| `PG_BETA_DECAY_RATE` | 0.995 | C1 | 半衰期 ~77min（平衡稳定与可塑） |
| `PG_BETA_DECAY_INTERVAL` | 1000 | C1 | ~33s @30Hz |
| `PG_CREDIT_HIT_SCALE` | 1.0 | C1 | 命中 = 1 伪观察 |
| `PG_CREDIT_MISS_SCALE` | 1.0 | C1 | 未中 = 1 伪观察 |
| `PG_BETA_INIT_TOTAL` | 10.0 | C1 | 遗留 strength 转换基数 |
| `PG_OBSERVE_HIT` | 0.5 | C1 | 观察 = 弱命中（非 post_hoc 验证） |
| `W_EPI_MAX` | 2.0 | C2 | 认识权重上限 |
| `PRIORITY_W_INFO_GAIN` | 1.5 | C3 | 信息增益在重放优先级中的权重 |

---

## 10. 研究引用

### 诺贝尔奖级

| 引用 | 年份 | 奖项 | 本文档相关性 |
|------|------|------|-------------|
| Kandel — 突触可塑性 | 2000 诺贝尔 | 生理学/医学 | 证据累积的分子基础 |
| O'Keefe / Moser — 位置/网格细胞 | 2014 诺贝尔 | 生理学/医学 | 正向重放 + 继任表征 |
| Hopfield — 联想记忆 | 2024 诺贝尔 | 物理学 | L4 能量最小化 |
| Kahneman — 双系统 | 2002 诺贝尔 | 经济学 | 反应式/前瞻式映射 |

### 直接驱动的关键论文

| 引用 | 年份 | 期刊 | 核心贡献 → 增补映射 |
|------|------|------|-------------------|
| Thompson | 1933 | Biometrika | 后验采样探索 → C1 |
| Russo et al. | 2018 | Found. & Trends ML | Thompson Sampling 理论保证 → C1 |
| Gershman | 2018 | Cognition | 人类探索 ≈ Thompson Sampling → C1 |
| Daw et al. | 2006 | Nature | 不确定性触发探索模式 → C1/C2 |
| Friston et al. | 2015 | Cognitive Neuroscience | 认识价值 + 实用价值 → C2 |
| Schwartenbeck et al. | 2015 | Cerebral Cortex | 多巴胺编码期望确定性 → C2 |
| Gottlieb et al. | 2013 | Trends Cognitive Sci. | 好奇心 = 认识价值 → C2 |
| Schmidhuber | 2010 | IEEE Trans. AMD | 好奇心 = 学习进度最大化 → C2 |
| Stachenfeld et al. | 2017 | Nature Neuroscience | 继任表征 = 转移矩阵 → C3 |
| Mattar & Daw | 2018 | Nature Neuroscience | 优先重放 = gain × need → C3 |
| Millidge et al. | 2020 | Neural Computation | EFE 沿采样轨迹可估 → C3 |
| Diba & Buzsáki | 2007 | Nature Neuroscience | 正向重放 = 规划 → C3 |
| Olshausen & Field | 1996 | Nature | 稀疏编码 = MAP → L2 已有 |
| Seeger | 2008 | Scholarpedia | 稀疏线性模型贝叶斯解释 → L2 |
| Bellec et al. | 2020 | Nature Communications | e-prop ≈ BPTT → 增补 A |
| Millidge et al. | 2021 | NeurIPS | PC ≈ BP → L4→L3 反传 |
| Bogacz | 2017 | J. Math. Psychology | PC = VFE 教程 → 理论框架 |
| Ernst & Banks | 2002 | Nature | 贝叶斯多感官整合 → L1 |

---

## 11. 风险登记

| 风险 | 影响 | 缓解 |
|------|------|------|
| **Beta 衰减节奏错误** | 太快→永远不确定不收敛；太慢→过度自信拒绝新证据 | DECAY_RATE=0.995 半衰期~77min 是保守起点；可通过遥测(mean variance across all relations)在线监控 |
| **认识价值主导决策** | W_EPI_MAX 太高→系统永远探索不利用 | 退火曲线保证成熟后 w_epi ≈ 0.10，且 babble floor 兜底 |
| **信息增益计算开销** | 每个 replay step 需查关联 relations | Phase 1 用回调默认返回 0.0；Phase 2 接入后单次查找 O(关系数)，在 REPLAY_CHAIN_MAX=32 下可接受 |
| **序列化版本兼容** | 旧 bin 没有 alpha/beta_param | 反序列化 fallback：从 strength 逆推 α=s×10, β=(1-s)×10 |
| **与增补 A 三因子的交互** | 同时修改 Relation 的学习规则可能产生意外 | 分离关注：增补 A 修改 elig 机制，C1 修改 strength 表示——汇合点(post_hoc_check_modulated)有明确接口 |

---

*补充文档结束。待业主审核后进入实施。*
