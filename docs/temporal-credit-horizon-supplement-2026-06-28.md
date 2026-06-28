# 时间信用分配地平线 — 补充设计文档

**日期**：2026-06-28
**版本**：v1
**状态**：DRAFT（补充件，不覆盖主 SDD）
**主文档**：`cognitive-middle-layers-sdd-2026-06-27.md`
**前置阅读**：主 SDD §1-§13、`perturbation-graph-phase-a-to-e-design-2026-06-18.md`

---

## 0. 文档定位

这是 `cognitive-middle-layers-sdd-2026-06-27.md` 的**独立补充件**，不修改主文档任何内容。

主 SDD 的五层架构解决了**空间维度的信用传递**（L4 匹配误差反传到 L3/L2/L1）。
本文档填补主 SDD 未覆盖的**时间维度信用分配**——当因果链跨越秒到分钟级延迟时，系统如何把结果归因到正确的历史动作。

**一句话**：大脑用级联 eligibility trace + 压缩重放 + 多通道第三因子三管齐下覆盖毫秒到小时的信用分配；当前系统只覆盖了毫秒级（post_hoc_check 33ms + PG_ELIG_TAU 600ms），秒-分钟级是架构空洞。

---

## 1. 问题陈述

### 1.1 当前系统的时间信用能力

| 机制 | 文件 | 时间窗口 | 局限 |
|------|------|----------|------|
| `post_hoc_check` | perturbation_graph.rs:1965 | 1 tick (~33ms) | 单槽，newest-firing-wins |
| `PG_ELIG_TAU_US` | perturbation_graph.rs:90 | 600ms (半衰期 415ms) | 纯指数衰减，无第三因子门控 |
| VPC cross-layer prior | vpc_cross_layer.rs | 1 跳（高→低） | prior 不参与学习规则，不递归 |

### 1.2 需要但缺失的时间尺度

真实认知学习中的因果链：

```
婴儿发声 "ba" (t=0s)
  → 无人响应 (t=0.3s)
    → 换发 "ma" (t=1.5s)            ← 基于 gap_pressure 驱动
      → 妈妈转头 (t=2.0s)
        → 妈妈笑 (t=3.5s)           ← ΔP 社会应答信用
          → 信用应回溯到 t=0 的 "ba 没用" 和 t=1.5 的 "换词"
```

这是 **4 跳、3.5 秒** 的因果链。当前系统 600ms 窗口 + 1-tick post_hoc_check 无法覆盖。

### 1.3 为什么调参解决不了

将 `PG_ELIG_TAU_US` 从 600ms 调到 10s：

- 10s 内 @30Hz = 300 个 tick，每个都有动作和事件在竞争信用
- 窗口越长 → 噪声越多 → 因果信号被不相关事件稀释
- 强化学习经典结论：延长指数衰减窗口 ≠ 解决信用分配问题

大脑的解法不是"延长窗口"——而是**压缩时间 + 选择性门控 + 多通道并行**。

---

## 2. 生物系统的四层时间信用机制

### 2.1 完整时间尺度分层

```
层级          时间窗口        生物机制                     诺贝尔/关键引用
────────────────────────────────────────────────────────────────────────
① 突触级       ~20ms         STDP 共放检测               Kandel 2000 诺贝尔
② 行为级       1-60s         三因子规则                   Gerstner 2018; Izhikevich 2007
                             (elig trace × 神经调制物)    Schultz/Dayan/Montague 1997
③ 情节级       分钟-小时     突触标记与捕获(STC)          Frey & Morris 1997
                             + 海马压缩重放(SWR)          Foster & Wilson 2006
                             + 时间细胞                   O'Keefe/Moser 2014 诺贝尔
④ 跨天级       小时-天       互补学习系统(CLS)            McClelland 1995
                             + 继任表征(SR)               Stachenfeld 2017; Hopfield 2024 诺贝尔
```

### 2.2 层级 ①：突触级共放检测 (~20ms)

**当前覆盖**：`post_hoc_check` (33ms) 近似此功能。

**核心原理**（Kandel 2000 诺贝尔）：
- 短期可塑性 (STP)：cAMP/PKA 通路，修改现有蛋白质，秒级
- 长期可塑性 (LTP)：需 CREB 转录因子 → 新蛋白质合成，小时→永久

**当前系统状态**：post_hoc_check 实现了基本的共放检测（relation 预测 vs 观察到的 anchor 比对），但是单槽设计（newest-firing-wins）丢失并行关系的信用。

### 2.3 层级 ②：行为级三因子规则 (1-60s) — **当前空洞**

**核心论文**：

**Schultz, Dayan & Montague 1997**, *Science*: "A Neural Substrate of Prediction and Reward"
- 中脑多巴胺神经元放电模式精确匹配 TD 误差：
  - 奖励 > 预期 → 放电增加（正 RPE）
  - 奖励 = 预期 → 基线（已学会）
  - 奖励 < 预期 → 放电抑制（负 RPE）
- 学习后多巴胺信号从奖励时刻**前移**到预测线索时刻 = TD 学习的生物实现

**Izhikevich 2007**, *Cerebral Cortex*: "Solving the Distal Reward Problem"
- STDP 检测前后突触共放 → 设置 eligibility trace（慢衰减，秒级）
- **关键：trace 对延迟期间的随机噪声活动不敏感**
- 多巴胺信号到达（1-3s 后）→ trace × 多巴胺 = 真正的权重变化
- 直接解决"动作在先、奖励延迟"的远端奖励问题

**Gerstner et al. 2018**, *Frontiers in Neural Circuits*: "Eligibility Traces and Plasticity on Behavioral Time Scales"
- 实验测量海马体 eligibility trace 窗口达 **~60 秒**
- 三因子规则公式化：`ΔW = elig(pre, post) × M(t)`
  - 因子 1：突触前活动
  - 因子 2：突触后活动
  - 因子 3：神经调制信号（多巴胺/去甲肾上腺素/血清素/乙酰胆碱）
- 前两因子设标记，第三因子决定"要不要真的改权重"

**Hong et al. 2022**, *Nature Communications*: "Norepinephrine potentiates and serotonin depresses visual cortical responses by transforming eligibility traces"
- **不只是多巴胺**——去甲肾上腺素(NE)和血清素(5-HT)也能转化 eligibility trace
- NE 增强（奖励/显著性），5-HT 压制（惩罚/脱离）
- 意味着三因子规则有**多个并行通道**

**级联 eligibility trace — Yin et al. 2025**, *arXiv*: "Learning From the Past with Cascading Eligibility Traces"
- 非单一时间常数，而是多层级联：
  - 快速迹（CaMKII 分子，~秒级）
  - 慢速迹（PKA/CREB 通路，~分钟级）
- 快迹衰减后慢迹仍在，覆盖更长时间窗口

### 2.4 层级 ③：情节级压缩重放 (分钟-小时) — **当前空洞**

**O'Keefe / Moser (2014 诺贝尔)**：位置细胞 + 网格细胞 = 空间记忆基础设施。

**Wilson & McNaughton 1994**, *Science*: 首次证明清醒时共放的海马神经元在后续睡眠中重新激活（重放）。

**Foster & Wilson 2006**, *Nature*: "Reverse replay of behavioural sequences in hippocampal place cells during the awake state"
- 位置细胞在清醒 Sharp-Wave Ripple (SWR, ~100ms) 中**反向重放**刚走过的路径
- 时间压缩比 ~100:1（10 秒真实经历 → 100ms 重放）
- 反向重放 = 从结果往回追溯原因 = **生物反向传播的时间等价物**

**Diba & Buzsáki 2007**, *Nature Neuroscience*:
- **正向重放**集中在行动前（841/887）= 前瞻/规划
- **反向重放**集中在行动后（395/464）= 回溯信用分配
- 前瞻 vs 回溯的不对称分布精确匹配两种计算需求

**Buzsáki 2015**, *Hippocampus* (115 页综述):
- SWR 是哺乳动物大脑最同步的群体模式
- 频率 140-180 Hz，持续 ~50-100ms
- **SWR 干扰实验证明因果性**：阻断 SWR → 记忆整合失败

**Mattar & Daw 2018**, *Nature Neuroscience*: "Prioritized memory access explains planning and hippocampal replay"
- 不是所有经历都应该等权重放——应该优先重放：
  - **高 gain**：新经历信息传播价值最大的（高 TD 误差）
  - **高 need**：即将面对的决策需要的
- 单一框架解释了：行动前正向重放、奖励后反向重放、远程但决策相关的重放

**突触标记与捕获 — Frey & Morris 1997**, *Nature*:
- 弱刺激 → 设"标签"（STP 级别，不需蛋白合成）
- 标签在 ~90 min 内等待"捕获"
- 强事件（多巴胺/NE = "这件事重要"）→ 合成蛋白 → 被标签捕获 → STP → LTP
- **Shetty et al. 2025**, *Communications Biology*: 在 strong-before-weak 范式下，捕获窗口可达 **9 小时**

**时间细胞 — MacDonald et al. 2011**, *Neuron*:
- 海马 CA1 神经元在延迟期间按特定时间点放电
- 群体编码 = "距离那件事过了多久"的主动时间标尺
- 跨度达 ~20 秒

**Theta 序列与相位进动 — O'Keefe & Recce 1993**, *Hippocampus*:
- 一个 theta 周期 (~125ms) 内压缩 ~1 秒的轨迹
- 前半段 = 过去位置，后半段 = 未来位置（预播）
- 给 STDP 提供了在生物可行的 ~20ms 窗口内建立序列关联的机会

### 2.5 层级 ④：跨天级慢整合 (小时-天)

**McClelland 1995**, *Psychological Review*: 互补学习系统 (CLS)
- 海马体：快速存储、一次学习（情节记忆）
- 新皮层：缓慢整合、多次重放（语义记忆）

**Stachenfeld, Botvinick & Gershman 2017**, *Nature Neuroscience*: "The hippocampus as a predictive map"
- 位置细胞编码**继任表征 (SR)**——不只是"我在哪"，而是"从这里出发未来会经过哪些状态"
- 价值分解：`V(s) = M(s) · w`，M = 状态转移预期，w = 即时奖励权重
- 奖励变了只更新 w，M 保持不变 → 高效泛化

**主 SDD 覆盖情况**：L4 `consolidate()` 设计（§7.2.4）已包含慢整合（每 10000 tick，相似模式合并，未访问模式衰减）。此层级无需本补充件重复设计。

---

## 3. 当前系统 vs 需求 的差距矩阵

| 时间层级 | 生物机制 | 当前系统 | 主 SDD 设计 | 本补充件 |
|----------|----------|----------|------------|----------|
| ① ~20ms | STDP | post_hoc_check (33ms) | 不涉及 | 不涉及（已够用） |
| ② 1-60s | 三因子规则 + 级联迹 | PG_ELIG_TAU=600ms, **无第三因子** | 不涉及 | **增补 A: 级联迹 + 多通道三因子** |
| ③ 分钟级 | 压缩重放 + STC | **完全缺失** | 不涉及 | **增补 B: L4 序列化 + 压缩重放** |
| ④ 小时-天 | CLS 慢整合 | — | L4 consolidate() | 不涉及（已设计） |

**增补 A 和 B 精确填补层级 ②③ 的空洞。**

---

## 4. 增补 A：级联 Eligibility Trace + 多通道三因子规则

### 4.1 科学依据（汇总）

- Izhikevich 2007: eligibility trace 对延迟期噪声不敏感 → 可安全延长到秒级
- Gerstner 2018: 实测 trace 窗口达 ~60s
- Hong 2022: NE/5-HT 各自独立转化 trace（非只有多巴胺）
- Yin 2025: 级联迹（快 + 慢）覆盖更宽时间尺度
- Bellec 2020 (e-prop): 三因子规则的严格数学形式化 = BPTT 的局部近似

### 4.2 数学规格

#### 4.2.1 级联 Eligibility Trace

替换当前的单一指数衰减：

```
当前:
  e(t) = exp(-dt / TAU_FAST)               # TAU_FAST = 600ms

级联设计:
  e_fast(t) = exp(-dt / TAU_FAST)           # TAU_FAST = 2_000ms (2s)
  e_slow(t) = exp(-dt / TAU_SLOW)           # TAU_SLOW = 30_000ms (30s)
  e(t)      = α · e_fast(t) + (1-α) · e_slow(t)

参数:
  TAU_FAST   = 2_000_000 us    # 快迹半衰期 ~1.4s，覆盖即时因果
  TAU_SLOW   = 30_000_000 us   # 慢迹半衰期 ~21s，覆盖跨秒因果
  α          = 0.7             # 快迹权重（近因优先）
```

TAU_FAST 从 600ms → 2s 的依据：Gerstner 2018 测量值最短为 0.3s（皮层），最长 60s（海马）。2s 是保守的皮层级选择。

#### 4.2.2 三因子学习规则

当前 perturbation_graph 的 relation strength 更新是二因子的：

```
当前（post_hoc_check）:
  if match:   strength += 0.01
  if mismatch: strength *= 0.95
```

三因子扩展：

```
Δstrength = elig(relation) × Σ_c (channel_c × gain_c)

四个调制通道 c:
  channel                  INTERO 来源                      gain 初始  方向
  ─────────────────────────────────────────────────────────────────────────
  DA (多巴胺等价)          INTERO_LEARNING_PROGRESS (152)    1.0       正向（学到东西=强化）
  NE (去甲肾上腺素等价)    INTERO_VITAL_ANXIETY (145)        0.5       正向（重要=标记）
  5HT (血清素等价)         INTERO_RESERVE (144)              0.3       负向（储备高=不急=弱化）
  ACh (乙酰胆碱等价)       INTERO_GAP_PRESSURE (150)         0.4       门控（高=重学, 低=巩固）

具体计算:
  mod_DA  = +gain_DA  × delta(learning_progress)    # LP 的变化量，非绝对值
  mod_NE  = +gain_NE  × vital_anxiety               # 绝对水平，焦虑时标记
  mod_5HT = -gain_5HT × saturate(reserve - 0.5)     # 储备充裕时压低信用
  mod_ACh = gain_ACh  × gap_pressure                 # 高 gap = 探索模式

  M(t) = clamp(1.0 + mod_DA + mod_NE + mod_5HT, 0.0, 3.0)
       × (if mod_ACh > θ_ach: 1.0 else: 0.3)       # ACh 门控：低 gap 时只保留 30%

  Δstrength = η × elig(relation) × M(t)
```

**关键设计选择**：
- `M(t) = 1.0` 当所有调制物中性时 → 退化到纯 elig trace 衰减式更新（向后兼容）
- ACh 是门控而非线性项 → 低不确定性时大幅抑制学习（巩固模式）
- 四通道独立 → 不同情境下不同类型的信用被强化/抑制

#### 4.2.3 标签与捕获 (STC) 简化实现

eligibility trace 衰减到低于阈值后不直接丢弃，而是设为"标签"状态：

```
if elig(relation) < ELIG_FLOOR (0.05) AND elig(relation) > 0.001:
    tag(relation) = true
    tag_expiry    = now + TAG_LIFETIME

if tag(relation) AND M(t) > CAPTURE_THRESHOLD (1.5):
    // 强调制信号"捕获"标签 → 恢复 eligibility
    elig(relation) = ELIG_REVIVE (0.3)
    tag(relation)  = false

参数:
  ELIG_FLOOR        = 0.05     # 低于此设标签
  TAG_LIFETIME      = 60s      # 标签存活时间（保守，生物 ~90min）
  CAPTURE_THRESHOLD = 1.5      # M(t) 超过此值触发捕获
  ELIG_REVIVE       = 0.3      # 捕获后恢复的 elig 值
```

这实现了 Frey & Morris 1997 的核心机制：弱事件设标签 → 后来的强事件（高多巴胺/NE）救活标签 → 学习发生。

### 4.3 Rust 结构

```rust
// 修改 perturbation_graph.rs

pub const PG_ELIG_TAU_FAST_US: f64 = 2_000_000.0;   // 2s
pub const PG_ELIG_TAU_SLOW_US: f64 = 30_000_000.0;  // 30s
pub const PG_ELIG_ALPHA: f64 = 0.7;                  // 快迹权重

pub const PG_ELIG_FLOOR: f64 = 0.05;
pub const PG_TAG_LIFETIME_US: u64 = 60_000_000;      // 60s
pub const PG_CAPTURE_THRESHOLD: f64 = 1.5;
pub const PG_ELIG_REVIVE: f64 = 0.3;

/// 扩展现有 edge/relation 结构
pub struct EligibilityState {
    pub elig_fast: f64,
    pub elig_slow: f64,
    pub tagged: bool,
    pub tag_expiry_us: u64,
}

impl EligibilityState {
    pub fn combined(&self) -> f64 {
        PG_ELIG_ALPHA * self.elig_fast + (1.0 - PG_ELIG_ALPHA) * self.elig_slow
    }

    pub fn decay(&mut self, dt_us: u64) {
        let dt = dt_us as f64;
        self.elig_fast *= (-dt / PG_ELIG_TAU_FAST_US).exp();
        self.elig_slow *= (-dt / PG_ELIG_TAU_SLOW_US).exp();

        let combined = self.combined();
        if !self.tagged && combined < PG_ELIG_FLOOR && combined > 0.001 {
            self.tagged = true;
            // tag_expiry 由调用者设为 now + TAG_LIFETIME_US
        }
    }

    pub fn try_capture(&mut self, modulation: f64) -> bool {
        if self.tagged && modulation > PG_CAPTURE_THRESHOLD {
            self.elig_fast = PG_ELIG_REVIVE;
            self.elig_slow = PG_ELIG_REVIVE * 0.5;
            self.tagged = false;
            return true;
        }
        false
    }

    pub fn fire(&mut self) {
        self.elig_fast = 1.0;
        self.elig_slow = 1.0;
        self.tagged = false;
    }
}

/// 四通道调制信号计算
pub struct ModulationChannels {
    pub da_gain: f64,   // 1.0
    pub ne_gain: f64,   // 0.5
    pub sht_gain: f64,  // 0.3
    pub ach_gain: f64,  // 0.4
    pub ach_gate_theta: f64,  // 0.3
}

impl ModulationChannels {
    pub fn compute(&self, lp_delta: f64, anxiety: f64, reserve: f64, gap: f64) -> f64 {
        let mod_da  =  self.da_gain  * lp_delta;
        let mod_ne  =  self.ne_gain  * anxiety;
        let mod_5ht = -self.sht_gain * (reserve - 0.5).max(0.0);
        let mod_ach =  self.ach_gain * gap;

        let base = (1.0 + mod_da + mod_ne + mod_5ht).clamp(0.0, 3.0);
        let gate = if mod_ach > self.ach_gate_theta { 1.0 } else { 0.3 };
        base * gate
    }
}
```

### 4.4 接入点

```
文件: perturbation_graph.rs (修改现有)
替换: decay_eligibility(), decay_relation_eligibility()
新增: EligibilityState 结构, ModulationChannels
读取: viewport 的 INTERO 轴值（learning_progress delta, vital_anxiety, reserve, gap_pressure）
调用时机: 每 tick，在 post_hoc_check 之后
```

### 4.5 向后兼容

当所有 INTERO 轴中性时 `M(t) ≈ 1.0`，退化到纯 eligibility trace 衰减行为。
新旧系统可通过 feature flag 切换（`USE_THREE_FACTOR: bool`），灰度上线。

---

## 5. 增补 B：L4 模式序列化 + 压缩重放

### 5.1 科学依据（汇总）

- Hopfield 1982 / Sompolinsky & Kanter 1986: 非对称权重 Hopfield 网络存储序列
- Foster & Wilson 2006: 反向重放 = 从结果往回追溯原因
- Diba & Buzsáki 2007: 正向重放(行动前/前瞻) vs 反向重放(行动后/信用)
- Mattar & Daw 2018: 优先重放——高 TD 误差 + 高决策相关性的经历优先
- Schaul et al. 2016: 计算领域的优先经验回放
- Ramesh et al. 2024 (ICML): 序列压缩——可预测步骤压缩掉，只保留决策点
- Asaad, Rainer & Miller 2017: PFC 在反馈时刻同时编码"之前的刺激"和"结果" = conjunction coding

### 5.2 两个子模块

#### 5.2.1 L4 模式转移矩阵（序列存储）

主 SDD 的 `PatternMemory` 存储静态模式（`[f64; PATTERN_DIM]`）。
增加转移矩阵记录"模式 A 之后常出现模式 B"：

```
数据结构:
  transitions: Vec<PatternTransition>

  PatternTransition {
      from_id: usize,          // 源模式
      to_id: usize,            // 目标模式
      strength: f64,           // 转移强度 (0.0..1.0)
      avg_dt_us: u64,          // 平均转移间隔
      count: u32,              // 观察次数
  }

记录时机:
  每次 L4 query() 返回 Completed → 记录 (prev_pattern_id, curr_pattern_id)
  prev_pattern_id 从上一 tick 的查询结果缓存

学习:
  if (from, to) 已存在:
      strength ← (1-α_tr) × strength + α_tr × 1.0     # α_tr = 0.05
      avg_dt_us ← EMA(actual_dt_us)
      count += 1
  else:
      创建新 transition, strength = 0.1

淘汰:
  strength < 0.01 且 count < 3 → 删除
  总数 > TRANSITION_CAP (8192) → 删除最弱的
```

#### 5.2.2 压缩重放引擎

```
触发条件（三选一）:
  (a) 静息态: HEALTH_QUIET 探测器激活（已存在于代码中）
  (b) 惊奇后: L4 query() 返回 Separated（新模式 = 值得复盘）
  (c) 社会应答后: ΔP 信用变化 > θ_replay_trigger

重放过程:

  1. 选择序列（优先排序）:
     recent_sequence = 从 transition 矩阵回溯最近 N 个模式
     priority(seq) = Σ_i |surprise_i| + max(|ΔP_social|)
     选择 priority 最高的序列

  2. 方向选择:
     if 触发 = 静息态:  正向重放（前瞻/规划）
     if 触发 = 惊奇/ΔP: 反向重放（信用分配）

  3. 压缩注入:
     for each pattern in sequence (正向/反向):
       // 跳过可预测步骤（AR-2 预测误差低的 = 可压缩）
       if predictability(pattern) > θ_compress: continue

       // 将模式重新注入 L2/L3 处理
       features  = pattern.vector    // 直接作为 L2 输出
       modulated = L3.modulate(features, current_context)
       result    = L4.query(modulated)
       // L4 匹配误差 → 梯度 → 更新 L3/L2/L1 权重

       // 三因子规则同时作用于相关 PG relations
       for relation in related_relations(pattern):
           relation.elig.fire()  // 重放 = 重新激活 elig
           // 当前 M(t) 调制信号会门控学习
```

### 5.3 数学规格

#### 5.3.1 优先排序

```
priority(transition_chain) = 
    Σ_{i∈chain} (
        w_surprise × |L4_mismatch_i|          # 惊奇度
      + w_social  × |ΔP_i|                    # 社会信用变化
      + w_recency × exp(-age_i / τ_recency)   # 新近性
    )

参数:
  w_surprise  = 1.0
  w_social    = 2.0     # 社交事件优先（与发育阶段相关）
  w_recency   = 0.5
  τ_recency   = 300s    # 5 分钟新近性窗口
```

#### 5.3.2 压缩

```
可压缩判定:
  predictability(pattern_i) = 1 - mean_surprise(pattern_i)

  mean_surprise 来自 pattern_i 对应时间窗口的 AR-2 平均预测误差

  θ_compress = 0.8      # 可预测度 > 80% 的步骤跳过

压缩效果:
  假设 30Hz, 4 秒序列 = 120 tick
  如果 80% 的 tick 可预测 → 压缩到 ~24 步
  24 步 × 压缩重放速度 ≈ 在 <1s 内完成
  → 在 elig trace 窗口内
```

#### 5.3.3 重放速度

```
真实时间: 1 tick = 10ms (100Hz)
重放时间: 1 重放步骤 = 1 tick（与正常处理同速）
         但跳过了可压缩步骤 → 有效压缩

生物参照: SWR 压缩比 5-20×
本设计有效压缩比: ~5× (80% 步骤跳过)
```

### 5.4 Rust 结构

```rust
// 新建 replay_engine.rs

pub const REPLAY_CHAIN_MAX: usize = 32;     // 最大重放序列长度
pub const REPLAY_COMPRESS_THETA: f64 = 0.8;  // 可压缩阈值
pub const REPLAY_TRIGGER_DP: f64 = 0.1;      // ΔP 触发阈值
pub const TRANSITION_CAP: usize = 8192;

pub struct PatternTransition {
    pub from_id: usize,
    pub to_id: usize,
    pub strength: f64,
    pub avg_dt_us: u64,
    pub count: u32,
}

pub enum ReplayDirection {
    Forward,   // 前瞻/规划
    Reverse,   // 信用分配
}

pub enum ReplayTrigger {
    Quiet,                    // HEALTH_QUIET 激活
    Surprise { pattern_id: usize },  // L4 Separated
    Social { dp_delta: f64 },        // ΔP 变化
}

pub struct ReplayEngine {
    pub transitions: Vec<PatternTransition>,
    pub prev_pattern_id: Option<usize>,

    /// 最近 N 个模式 ID 的环形缓冲（用于回溯）
    pub recent_trace: VecDeque<(usize, u64)>,  // (pattern_id, timestamp_us)
}

impl ReplayEngine {
    /// 记录模式转移（每次 L4 Completed 后调用）
    pub fn record_transition(&mut self, pattern_id: usize, t_us: u64);

    /// 检查是否应该触发重放
    pub fn should_replay(&self, trigger: ReplayTrigger) -> bool;

    /// 执行一次重放，返回需要重新注入的模式序列
    pub fn generate_replay(
        &self,
        trigger: ReplayTrigger,
        predictability: &dyn Fn(usize) -> f64,  // pattern_id → 可预测度
    ) -> Vec<(usize, ReplayDirection)>;

    /// 回溯信用链（反向遍历 transition 矩阵）
    pub fn trace_back(
        &self,
        from_id: usize,
        max_hops: usize,
    ) -> Vec<usize>;  // 反向模式序列
}
```

### 5.5 与主 SDD L4 的接口

```
主 SDD L4 PatternMemory
  ├── query() → 返回 Completed/Separated/Ambiguous  [不变]
  ├── consolidate()                                   [不变]
  └── seed_from_fast_path()                           [不变]

本补充件 ReplayEngine
  ├── record_transition() ← 被 L4 query(Completed) 后调用
  ├── should_replay()     ← 被 orchestrator 每 tick 检查
  └── generate_replay()   → 返回模式序列 → 重新注入 L2/L3/L4 管线
```

ReplayEngine 是 L4 PatternMemory 的**侧车**，不修改 PatternMemory 的任何内部结构。

### 5.6 接入点

```
新建文件: replay_engine.rs
调用者: orchestrator 主循环（在 L4 query 之后、下一 tick 之前）
读取: L4 query 结果、AR-2 预测误差、INTERO 轴值、HEALTH_QUIET 状态
写入: 重放时重新注入 L2/L3/L4 管线 + 更新 PG relation strength（通过增补 A 的三因子规则）
```

---

## 6. 增补 A + B 的联合数据流

```
正常 tick:
  L0 → L1 → L2 → L3 → L4.query()
                         │
                         ├── Completed → ReplayEngine.record_transition()
                         │               PG relations: elig.decay() + M(t) 三因子
                         ├── Separated → ReplayEngine.should_replay(Surprise)? 
                         └── Ambiguous → 注意力聚焦（主 SDD 已设计）

社会应答到达:
  ΔP 信用变化 → ReplayEngine.should_replay(Social)?

静息态:
  HEALTH_QUIET → ReplayEngine.should_replay(Quiet)?

重放 tick (插入正常 tick 之间):
  ReplayEngine.generate_replay()
    → for each pattern in sequence:
        if predictability > θ: skip (压缩)
        else:
          L3.modulate(pattern.vector, current_context)
          → L4.query(modulated)
          → L4 匹配误差 → 梯度 → L3/L2/L1 权重更新
          → PG relation.elig.fire()    ← 重放重新激活 elig
          → 三因子: Δstrength = elig × M(t)
```

---

## 7. 与主 SDD 设计的交互建议

以下是对主 SDD 的**修改建议**（不在本文档中执行，待业主代码审核后决定是否纳入主 SDD）：

### 7.1 建议 S1：L1 画布增加"上次广播缓存"

**依据**：Asaad, Rainer & Miller 2017 证明 PFC 在反馈时刻同时编码"之前的刺激"和"结果"（conjunction coding）。

**建议**：L1 SharedCanvas 增加 `prev_broadcast: [f64; CANVAS_DIM]`，保存上一次点火的广播模式。当 ΔP 社会应答或其他延迟反馈到达时，系统可以同时访问"当时广播了什么"和"现在收到了什么"。

**主 SDD 影响**：§4.4 SharedCanvas 结构增加一个字段，step() 方法末尾保存当前 ignited 模式到 prev_broadcast。低风险。

### 7.2 建议 S2：L3 FiLM 的四通道应同时服务信用分配

**依据**：Hong et al. 2022 证明 NE/5-HT 各自独立转化 eligibility trace。

**建议**：主 SDD §6.2 的 L3 context 向量（8 个 INTERO 轴）不仅用于调制特征（现有设计），还应通过增补 A 的 ModulationChannels 同时驱动 PG relation 的三因子学习。即 L3 读取的同一组 INTERO 值，同时服务两个目的：(a) FiLM 调制感知 (b) 三因子门控学习。

**主 SDD 影响**：§6 增加一段说明 context 向量的双重角色。无结构变更。

### 7.3 建议 S3：L4 PatternMemory 考虑预留转移接口

**依据**：增补 B 的 ReplayEngine 需要从 L4 获取模式 ID 和转移信息。

**建议**：§7.4 PatternMemory 结构预留 `pub fn last_matched_id(&self) -> Option<usize>` 接口，供 ReplayEngine 记录转移。

**主 SDD 影响**：一个 getter 方法。最低风险。

### 7.4 建议 S4：Phase 5 端到端梯度考虑时间维度

**依据**：主 SDD §10.2 Phase 5 的端到端梯度是单 tick 内的空间梯度。增补 B 的重放机制会在重放 tick 中产生额外的梯度信号。

**建议**：Phase 5 实施时，重放 tick 产生的 L4 匹配误差应与正常 tick 的误差使用相同的梯度路径（L4→L3→L2→L1），但学习率可以不同（重放梯度可以打折，如 `η_replay = 0.5 × η_normal`）。

**主 SDD 影响**：Phase 5 备注。不影响 Phase 1-4。

---

## 8. 实施计划

### 8.1 依赖顺序

```
增补 A（级联迹 + 三因子）──→ 增补 B（序列 + 重放）
        │                              │
        └── 独立于主 SDD Phase 1-4 ────┘
            可并行开发
            增补 B 的重放注入需要 L2/L3 存在（主 SDD Phase 2-4 之后）
            增补 A 可立即开发（只修改 perturbation_graph.rs）
```

### 8.2 分阶段

| 阶段 | 内容 | 文件 | 可并行于主 SDD |
|------|------|------|---------------|
| **T1** | 级联 elig trace（替换单一衰减） | perturbation_graph.rs | 是，Phase 1 前即可 |
| **T2** | 四通道调制信号计算 | perturbation_graph.rs | 是 |
| **T3** | 三因子学习规则（elig × M(t)） | perturbation_graph.rs | 是 |
| **T4** | 标签与捕获 (STC) | perturbation_graph.rs | 是 |
| **T5** | L4 转移矩阵 | replay_engine.rs + pattern_memory.rs 接口 | 需 L4 存在（Phase 3 后） |
| **T6** | 压缩重放引擎 | replay_engine.rs | 需 L2/L3 存在（Phase 4 后） |

**T1-T4 可以在主 SDD 任何 Phase 之前独立开发**，因为只修改已存在的 perturbation_graph.rs。

### 8.3 验证标准

| 阶段 | 通过标准 |
|------|---------|
| T1 | 级联迹半衰期实测：快迹 ~1.4s，慢迹 ~21s；combined() 在 30s 后仍 > 0.01 |
| T2 | 四通道输出在 INTERO 轴全中性时 M(t) ≈ 1.0；单通道极值时 M(t) 在 [0.0, 3.0] 范围 |
| T3 | 有第三因子时 relation strength 变化 vs 无第三因子时对比：信号/噪声比提升 > 2× |
| T4 | 标签设置后 30s 内强调制信号到达 → 标签被捕获 → relation strength 更新 |
| T5 | 连续 Completed 查询记录转移；回溯 trace_back 返回正确的反向序列 |
| T6 | 惊奇触发后反向重放序列正确；压缩跳过 > 80% 可预测步骤 |

---

## 9. 参数总表

| 参数 | 值 | 所在增补 | 理由 |
|------|-----|---------|------|
| `PG_ELIG_TAU_FAST_US` | 2,000,000 (2s) | A | Gerstner 2018 皮层实测最短 0.3s，2s 保守 |
| `PG_ELIG_TAU_SLOW_US` | 30,000,000 (30s) | A | 海马实测达 60s，30s 折中 |
| `PG_ELIG_ALPHA` | 0.7 | A | 近因优先 |
| `PG_ELIG_FLOOR` | 0.05 | A | 低于此设标签 |
| `PG_TAG_LIFETIME_US` | 60,000,000 (60s) | A | 生物 ~90min，保守取 60s |
| `PG_CAPTURE_THRESHOLD` | 1.5 | A | M(t) 超此值捕获标签 |
| `PG_ELIG_REVIVE` | 0.3 | A | 捕获后恢复的 elig 值 |
| `DA_GAIN` | 1.0 | A | 多巴胺通道增益 |
| `NE_GAIN` | 0.5 | A | 去甲肾上腺素通道增益 |
| `5HT_GAIN` | 0.3 | A | 血清素通道增益 |
| `ACH_GAIN` | 0.4 | A | 乙酰胆碱通道增益 |
| `ACH_GATE_THETA` | 0.3 | A | ACh 门控阈值 |
| `REPLAY_CHAIN_MAX` | 32 | B | 最大重放序列长度 |
| `REPLAY_COMPRESS_THETA` | 0.8 | B | 可压缩判定阈值 |
| `REPLAY_TRIGGER_DP` | 0.1 | B | ΔP 触发重放的阈值 |
| `TRANSITION_CAP` | 8192 | B | 转移矩阵容量上限 |
| `W_SURPRISE` | 1.0 | B | 重放优先权：惊奇度权重 |
| `W_SOCIAL` | 2.0 | B | 重放优先权：社交权重 |
| `W_RECENCY` | 0.5 | B | 重放优先权：新近性权重 |
| `τ_RECENCY` | 300s | B | 新近性衰减时间 |

---

## 10. 研究引用

### 诺贝尔奖级

| 引用 | 年份 | 奖项 | 本文档相关性 |
|------|------|------|-------------|
| Kandel — 突触可塑性的分子机制 | 2000 诺贝尔 | 生理学/医学 | STP/LTP 时间尺度 = elig trace 的分子基础 |
| O'Keefe / Moser — 位置细胞 + 网格细胞 | 2014 诺贝尔 | 生理学/医学 | 海马重放的基础设施 |
| Hopfield — 联想记忆网络 | 2024 诺贝尔 | 物理学 | 序列 Hopfield = 转移矩阵的理论基础 |

### 关键论文

| 引用 | 年份 | 期刊 | 核心贡献 |
|------|------|------|---------|
| Schultz, Dayan & Montague | 1997 | Science | 多巴胺 = TD 误差 |
| Frey & Morris | 1997 | Nature | 突触标记与捕获 |
| Izhikevich | 2007 | Cerebral Cortex | elig trace 解决远端奖励问题 |
| Foster & Wilson | 2006 | Nature | 反向重放 |
| Diba & Buzsáki | 2007 | Nature Neuroscience | 正向/反向重放不对称 |
| MacDonald et al. | 2011 | Neuron | 时间细胞 |
| O'Keefe & Recce | 1993 | Hippocampus | Theta 相位进动 |
| Buzsáki | 2015 | Hippocampus | SWR 综述 (115p) |
| Gerstner et al. | 2018 | Front. Neural Circuits | 三因子规则实验证据 |
| Mattar & Daw | 2018 | Nature Neuroscience | 优先重放的规范理论 |
| Bellec et al. (e-prop) | 2020 | Nature Communications | 三因子规则的严格数学化 |
| Hong et al. | 2022 | Nature Communications | NE/5-HT 独立转化 elig trace |
| Stachenfeld et al. | 2017 | Nature Neuroscience | 继任表征 |
| Asaad, Rainer & Miller | 2017 | J. Neuroscience | PFC conjunction coding |
| Heo & Lee | 2020 | Scientific Reports | PFC 中实测 elig trace 维持 |
| Miller & Cohen | 2001 | Annu. Rev. Neuroscience | PFC 偏置信号整合理论 |
| Yin et al. | 2025 | arXiv | 级联 eligibility trace |
| Shetty et al. | 2025 | Communications Biology | STC 9 小时窗口 |
| Liu et al. | 2021 | PNAS | ~100 种神经肽 = 并行信用通道 |
| Ni et al. | 2023 | NeurIPS (Oral) | Transformer attention ≠ 信用分配 |
| Ramesh et al. | 2024 | ICML | 序列压缩加速信用传播 |
| Schaul et al. | 2016 | ICLR | 优先经验回放 |
| McClelland et al. | 1995 | Psychological Review | 互补学习系统 |
| Sompolinsky & Kanter | 1986 | Phys. Rev. Lett. | 序列 Hopfield 网络 |
| Pignatelli et al. | 2024 | TMLR | 时间信用分配综述 |

---

## 11. 风险登记

| 风险 | 影响 | 缓解 |
|------|------|------|
| **三因子噪声放大** | 多通道调制信号在极端 INTERO 状态下 M(t) 过大 → 不稳定 | clamp(0.0, 3.0) + ACh 门控 + 灰度上线 |
| **重放干扰正常处理** | 重放 tick 与正常感知 tick 竞争 CPU 时间 | 重放只在静息态/事件后触发，非每 tick；重放步骤有上限 (32) |
| **标签内存开销** | 每个 relation 增加 tagged + tag_expiry 字段 | 仅 2 字段 (bool + u64)，在 65536 relations 下 < 1MB |
| **转移矩阵膨胀** | 模式数 × 模式数 的潜在组合爆炸 | TRANSITION_CAP = 8192 硬上限 + 弱转移淘汰 |
| **压缩阈值过激** | θ_compress 太低 → 几乎不压缩；太高 → 跳过关键步骤 | 初始 0.8 保守；后期可做发育退火 |
| **重放时 L3 context 是当前的而非历史的** | 回溯重放用当前情绪调制历史模式 → 偏差 | 这是设计选择非 bug：当前情绪影响"如何重新评价过去"= 情绪依赖性记忆重整合 (Nader 2003) |

---

*补充文档结束。待业主代码审核后决定是否纳入主 SDD 或独立执行。*
