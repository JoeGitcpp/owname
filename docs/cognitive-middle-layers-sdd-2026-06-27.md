# 认知中间层架构 — 软件设计文档 (SDD)

**日期**：2026-06-27
**版本**：v1
**状态**：REVIEW（锁定设计，未动工）
**前置阅读**：`perturbation-graph-phase-a-to-e-design-2026-06-18.md`、`prospection-emergence-scaffold-sdd-2026-06-27.md`

---

## 0. 文档定位

这是**设计先行文档**——记录认知中间层（感官输入 → 预测输出之间的处理栈）的架构决策。
来源于 2026-06-27 对话中对以下问题的系统性讨论与研究调研。

**一句话**：167 轴感官输入和 AR-2 预测头之间缺少整个认知处理栈——五层中间层基于人类神经科学关键成果设计，填补从感觉到认知的空白。

---

## 1. 问题陈述

### 1.1 当前架构的空洞

现有系统：

```
167 轴感官输入（lib.rs:87-221）
  16 VIB（听觉频带）+ 128 VIS（4×4×8 视觉网格）
  15 INTERO（内感受）+ 5 CODE（数字对象）+ 2 GAUSS + 1 DEFENSE
      │
      ├─ 注意力（viewport.rs:347, active_surface.rs, prediction_pressure_field.rs）
      ├─ perturbation_graph（24-D sig 节点 + 类型化关系 + FastPath）
      │
      ↓
167 个独立 AR-2 标量预测器（predictor.rs）@ 100Hz
  + VPC cross-layer 稀疏先验（vpc_cross_layer.rs）
  → 惊奇信号 → 发育时钟
```

**问题**：轴值几乎直接进入预测——没有中间的特征提取、模式识别、上下文调制。等于有眼睛有反射弧，但没有皮层。

### 1.2 对话中识别的六个缺陷

| # | 缺陷 | 表现 | 根因 |
|---|---|---|---|
| 1 | **线性天花板** | 泼溅画布只做线性投射，无法解决 XOR 类非线性组合问题 | 缺非线性特征提取层 |
| 2 | **跨视角不变性缺失** | 不知道「这几次看到的是同一个东西」 | 缺模式库 + 吸引子补全 |
| 3 | **认知搜索无上下文偏置** | 性格/经验/情绪不影响「看到什么」 | 缺调制层 |
| 4 | **认识 vs 行动混淆** | AR-2 同时预测感觉和动作，无分离 | 架构未分层 |
| 5 | **输入分辨率无发育退火** | 167 轴从第一天全精度，婴儿应从模糊开始 | 缺发育性降采样 |
| 6 | **偏置写死而非可学习** | 调制因子应是架构参数，成长中自调 | 缺 meta-learning 机制 |

### 1.3 设计原则（业主决策）

- **注意力是唯一的**，不是碎片化的。现有多套注意力相关结构最终需统一到一个机制。
- **路线 A 纯涌现**：不手写分类器、不手工设计滤波器，搭建条件让特征自组织涌现。
- **架构可学习**：因子（性格/经验/环境）抽象为架构参数，成长中自调，不写死。
- **粗到细发育**：输入分辨率、特征复杂度、模式库容量都从低到高渐进。

---

## 2. 五层架构总览

```
167 轴 ──→ [L0 发育降采样] ──→ 有效输入 N_eff(t) ≤ 167

                ┌─────────────────────────────┐
           L1   │   共享画布 (Shared Canvas)    │  多模态汇聚 + 竞争广播
                │   Baars 1988 / Dehaene 2011  │  脑区 = 丘脑 + 前额叶
                └──────────────┬──────────────┘
                               │
                ┌──────────────┴──────────────┐
           L2   │   非线性特征提取              │  稀疏编码 + 除法归一化
                │   Olshausen 1996 / Hubel 62  │  脑区 = V1-V4 层级
                │   Carandini & Heeger 2012    │
                └──────────────┬──────────────┘
                               │
                ┌──────────────┴──────────────┐
           L3   │   FiLM 上下文调制             │  γ(state) × features + β(state)
                │   Salinas & Abbott 2001      │  脑区 = 神经调制系统
                │   Perez 2018                 │
                └──────────────┬──────────────┘
                               │
                ┌──────────────┴──────────────┐
           L4   │   模式库 + 补全 + 分离        │  Modern Hopfield 检索
                │   Hopfield 1982/2024 Nobel   │  脑区 = 海马 CA3 + DG
                │   Ramsauer 2020              │
                │   McClelland 1995 CLS        │
                └──────────────┬──────────────┘
                               │
                ┌──────────────┴──────────────┐
           存在  │   AR-2 预测 + 惊奇 + 时钟    │  predictor.rs + vpc*.rs
                │   (Rao & Ballard 1999)       │
                └─────────────────────────────┘
```

---

## 3. L0 — 发育降采样

### 3.1 科学依据

- 新生儿视力 ~20/400（成人标准法定失明），3-6 月内迅速提高
- Elman 1993 "Starting Small"：从简化输入开始学习效果优于完整输入
- Turkewitz & Kenny 1982 成熟性约束假说：早期感官能力有限是 feature 非 bug
- 与现有 `AR2_LR_SCALE_NAIVE`（学习率从 3.0 退火到 1.0）同构，扩展到输入层

### 3.2 数学规格

```
N_eff(maturity) = N_floor + (AXIS_COUNT - N_floor) × sigmoid(k × (maturity - m₀))

参数:
  N_floor     = 32      # 新生期可用轴数（听觉全开 + 视觉仅亮度/运动）
  AXIS_COUNT  = 167     # 完全成熟时
  k           = 8.0     # sigmoid 陡度
  m₀          = 0.3     # 半开放成熟度
```

### 3.3 轴开放优先级

| 优先级 | 轴组 | 数量 | 理由 |
|--------|------|------|------|
| 1（新生） | VIB 全部 + VIS_BRIGHTNESS + VIS_MOTION + INTERO 核心 5 | ~32 | 听觉+光暗+运动=婴儿最初能力 |
| 2（早期） | VIS_EDGE_ENERGY + VIS_TEMPORAL_FLUX + 剩余 INTERO | ~50 | 边缘+时变=形状感知前驱 |
| 3（中期） | VIS_R/G/B + VIS_SATURATION | ~80 | 色彩=需要视锥成熟 |
| 4（晚期） | CODE_AXES + GAUSS_AXES + 所有剩余 | 167 | 高阶/数字感知 |

### 3.4 实现方式

未开放的轴：画布投射权重钳零（`W_proj[axis] = 0`），AR-2 预测器正常跑但不影响下游。
不删轴、不改 AXIS_COUNT——只是画布看不到被遮蔽的轴。

### 3.5 接入点

```
文件: 新建 developmental_gate.rs
接口:
  pub fn effective_mask(maturity: f64) -> [bool; AXIS_COUNT]
  pub fn axis_gain(maturity: f64, axis: usize) -> f64   // 0.0..1.0 渐开
被调用: L1 共享画布投射前
```

---

## 4. L1 — 共享画布 (Shared Canvas)

### 4.1 科学依据

**Global Workspace Theory（Baars 1988，Dehaene & Changeux 2011）**
- 各专家模块独立处理，少数胜者广播到全局工作空间
- 点火（ignition）：递归自激超过侧抑制阈值 → 双稳态跳变 → 全局广播
- 广播后所有模块可读 = 意识的计算基底

**Convergence-Divergence Zones（Damasio 2010）**
- 层级式汇聚，不是一步到位
- 低层汇聚相邻模态，高层汇聚更远组合

**贝叶斯多感官整合**
- `P(S|V,A) ∝ P(V|S)·P(A|S)·P(S)`，按可靠性（逆方差）加权各模态

### 4.2 数学规格

#### 4.2.1 模态投射

将 167 轴按模态分为 M 个模态组（M=5: VIB, VIS, INTERO, CODE, GAUSS+DEFENSE）。

每个模态 m 有投射矩阵 `W_m ∈ R^{D_canvas × N_m}`：

```
z_m = W_m · x_m       # 模态 m 的轴值投射到画布空间
```

画布维度 `D_canvas = 64`（设计参数，可调）。

#### 4.2.2 可靠性加权融合

```
precision_m = 1 / (var(err_m) + ε)       # 模态精度 = 预测误差方差的倒数
                                          # err 从现有 AR-2 惊奇直接取

z_fused = Σ_m (precision_m × z_m) / Σ_m precision_m
```

这是贝叶斯最优多感官融合的精确实现。精度高的模态（预测误差小 = 可靠）权重大。

#### 4.2.3 竞争广播（点火）

画布上 D_canvas 维中只有 K 个维度「点火」广播：

```
动力学:
  da_i/dt = -a_i + ReLU(w_self · a_i - w_inhib · Σ_{j≠i} a_j + z_fused_i - θ)

稳态近似（每 tick 一步欧拉）:
  a_i ← (1-dt/τ) · a_i + (dt/τ) · ReLU(w_self · a_i - w_inhib · mean(a) + z_fused_i - θ)

参数:
  w_self  = 1.2        # 自激（> 1 才能点火）
  w_inhib = 0.15       # 侧抑制
  θ       = 0.3        # 点火阈值
  τ       = 50ms       # 时间常数
  K       = 16         # 最大同时点火维度
```

点火是自发涌现的——不需要外部开关。信号弱 → 被抑制衰减；信号强 → 自激超过阈值 → 锁存广播。

### 4.3 与现有代码的关系

- **`GlobalAttentionState`（viewport.rs:347）**：当前是 8-slot 计分板。L1 画布替代其「竞争选择」功能。GlobalAttentionState 的 attention_shares 可以从画布点火模式派生，而非独立计算。
- **`ActiveFocusTrace`（active_surface.rs）**：其 pressure/resonance 可以作为画布侧抑制的上下文输入（通过 L3 FiLM），而非独立注意力系统。
- **`PredictionPressureField`（prediction_pressure_field.rs）**：其 dominant wave 选择逻辑可并入画布竞争。
- **关键**：三者统一到画布竞争，注意力 = 画布上的点火模式。不是三套系统。

### 4.4 Rust 结构

```rust
// 新建 shared_canvas.rs

pub const CANVAS_DIM: usize = 64;
pub const MODALITY_COUNT: usize = 5;
pub const IGNITION_K: usize = 16;

pub struct SharedCanvas {
    /// 各模态投射矩阵 W_m: [D_canvas × N_m]
    pub proj_weights: [Vec<Vec<f64>>; MODALITY_COUNT],

    /// 画布激活状态
    pub activation: [f64; CANVAS_DIM],

    /// 各模态精度（从 AR-2 惊奇 EMA 倒数）
    pub modality_precision: [f64; MODALITY_COUNT],

    /// 竞争动力学参数
    pub w_self: f64,     // 1.2
    pub w_inhib: f64,    // 0.15
    pub theta: f64,      // 0.3
    pub tau_ms: f64,     // 50.0

    /// 点火掩码（哪些维度当前在广播）
    pub ignited: [bool; CANVAS_DIM],
}

impl SharedCanvas {
    /// 每 tick 调用：投射 → 融合 → 竞争 → 广播
    pub fn step(
        &mut self,
        axes: &[f64; AXIS_COUNT],
        dev_mask: &[f64; AXIS_COUNT],  // L0 发育增益
        dt_ms: f64,
    ) -> [f64; CANVAS_DIM];  // 返回广播后的画布状态
}
```

### 4.5 投射矩阵学习

`W_m` 不手工设计。初始化为小随机值，通过 L2 层的稀疏重构误差反向传播梯度更新：

```
dW_m/dt = -α · ∂E_sparse / ∂W_m
```

即：投射矩阵由「能否用最少的画布维度重构原始输入」驱动自组织。

---

## 5. L2 — 非线性特征提取

### 5.1 科学依据

**Hubel & Wiesel（1962，诺贝尔奖）**
- 简单细胞 = 特定朝向的边缘检测（线性滤波 + 整流）
- 复杂细胞 = 多个简单细胞的非线性池化（位移不变性）

**Olshausen & Field（1996，稀疏编码）**
- 最小化 `‖input - Σ aᵢ·φᵢ‖² + λ·Σ|aᵢ|`
- 稀疏约束自动涌现 Gabor 样滤波器 = V1 简单细胞
- **不需要手工设计特征——稀疏性本身迫使系统学出有意义的特征**

**Carandini & Heeger（2012，除法归一化）**
- "大脑的标准计算"：`z = yⁿ / (σⁿ + Σ pₖ·yₖⁿ)`
- 响应相对于局部上下文，不是绝对强度
- 在暗房和亮房都能看清 = 自动适应局部统计量

**Rao & Ballard（1999，预测编码）**
- V1 不是特征检测器，是预测机器
- 高层预测低层，低层只上传预测误差
- 正负误差分开编码 = ReLU 的生物原型

### 5.2 数学规格

L2 对画布输出做三步变换：

#### Step 1: 稀疏编码（简单细胞）

```
目标: min_a ‖canvas - D·a‖² + λ·‖a‖₁

其中:
  canvas ∈ R^{D_canvas}         # L1 画布输出
  D ∈ R^{D_canvas × N_features} # 字典矩阵（可学习）
  a ∈ R^{N_features}            # 稀疏系数 = 特征激活
  λ = 0.1                       # 稀疏惩罚强度

推断（ISTA，每 tick 固定步数）:
  a ← shrink(a + α·Dᵀ·(canvas - D·a), λ·α)
  shrink(x, t) = sign(x)·max(|x|-t, 0)    # 软阈值 = L1 的近端算子

字典学习（慢，每 N tick 一次）:
  D ← D + η·(canvas - D·a)·aᵀ             # 赫布学习

参数:
  N_features = 128       # 特征数（> D_canvas = 过完备）
  ISTA_steps = 5         # 每 tick 推断步数
  α = 0.02               # 推断步长
  η = 0.001              # 字典学习率
```

#### Step 2: 能量池化（复杂细胞）

```
将 N_features 分为 G 个池化组（每组 P 个特征）:

  c_g = sqrt(Σ_{i∈group_g} a_i²)      # 能量池化

参数:
  G = 32            # 池化后特征数
  P = 4             # 每组大小（128/32）

池化组初始随机分配，可通过相关性聚类重组织。
```

#### Step 3: 除法归一化

```
  z_g = c_g² / (σ² + (1/G)·Σ_k c_k²)

参数:
  σ = 0.01          # 半饱和常数（防除零 + 设定动态范围）

输出: z ∈ R^G = 归一化后的特征向量
```

### 5.3 Rust 结构

```rust
// 新建 feature_extraction.rs

pub const N_FEATURES: usize = 128;
pub const N_POOLS: usize = 32;
pub const POOL_SIZE: usize = N_FEATURES / N_POOLS;  // 4
pub const ISTA_STEPS: usize = 5;

pub struct FeatureExtractor {
    /// 字典矩阵 D: [CANVAS_DIM × N_FEATURES]
    pub dictionary: Vec<[f64; N_FEATURES]>,  // CANVAS_DIM rows

    /// 稀疏系数（上一 tick 热启动）
    pub coefficients: [f64; N_FEATURES],

    /// 池化组分配
    pub pool_assignment: [usize; N_FEATURES],  // feature_i → group_id

    /// 除法归一化半饱和常数
    pub sigma_sq: f64,

    /// 归一化后输出
    pub output: [f64; N_POOLS],
}

impl FeatureExtractor {
    /// 每 tick: canvas → 稀疏推断 → 池化 → 归一化
    pub fn step(&mut self, canvas: &[f64; CANVAS_DIM]) -> [f64; N_POOLS];

    /// 慢速字典学习（每 100 tick）
    pub fn update_dictionary(&mut self, canvas: &[f64; CANVAS_DIM]);
}
```

### 5.4 涌现性质

- 字典 D 的列会自组织成有意义的特征基（听觉→频率调谐，视觉→边缘/运动检测）
- 不需要预先知道「什么特征有用」——稀疏性 + 重构误差自动决定
- 与现有 perturbation_graph 的 24-D sig 不冲突：sig 是手工定义的节点身份，L2 特征是自组织的画布表示

---

## 6. L3 — FiLM 上下文调制

### 6.1 科学依据

**Gain Modulation（Salinas & Abbott 2001）**
- 大脑最普遍的计算之一：`output = f(input) × g(context)`
- 乘法缩放不改变调谐曲线形状，只改变增益
- 决定「对什么敏感」而非「看到什么」

**神经调制四大系统**

| 调制物 | 计算等价 | 系统中的来源 |
|--------|----------|-------------|
| **多巴胺** | 奖励相关信号增益 `γ = 1 + DA` | `INTERO_LEARNING_PROGRESS`（学到东西 → 更关注学习相关模式） |
| **去甲肾上腺素** | softmax 温度 τ（高→探索，低→聚焦） | `INTERO_GAP_PRESSURE`（高间隙压力 → 广泛搜索） |
| **血清素** | 时间折扣（高→耐心） | `INTERO_RESERVE`（储备高 → 不急） |
| **乙酰胆碱** | 精度权重（增强自下而上，减弱自上而下） | `INTERO_VITAL_ANXIETY`（焦虑高 → 更依赖感官，少依赖预测） |

**FiLM（Perez 2018）**
- `FiLM(F) = γ · F + β`，其中 `[γ, β] = h(context)`
- `γ=1, β=0` 时恒等变换（中性 = 新生儿无偏好）
- `h` 是可学习的小网络，成长过程中参数漂移 = 个性形成

### 6.2 数学规格

```
输入:
  features ∈ R^{N_POOLS}     # L2 输出
  context  ∈ R^{D_ctx}       # 内部状态向量

上下文向量构成（从现有 INTERO 轴 + 发育状态）:
  context = [
    intero_reserve,           # INTERO_RESERVE (144)
    intero_vital_anxiety,     # INTERO_VITAL_ANXIETY (145)
    intero_agency,            # INTERO_AGENCY (148)
    intero_gap_pressure,      # INTERO_GAP_PRESSURE (150)
    intero_learning_progress, # INTERO_LEARNING_PROGRESS (152)
    intero_intention_pressure,# INTERO_INTENTION_PRESSURE (157)
    maturity,                 # 发育时钟
    arousal,                  # 唤醒度（若有）
  ]
  D_ctx = 8

调制网络:
  h1 = ReLU(W1 · context + b1)      # W1 ∈ R^{16 × D_ctx}
  γ  = 1 + W2_γ · h1                # W2_γ ∈ R^{N_POOLS × 16}, 初始化零
  β  = W2_β · h1                     # W2_β ∈ R^{N_POOLS × 16}, 初始化零

调制:
  modulated = γ ⊙ features + β       # ⊙ = 逐元素乘法

关键: γ 初始化在 1 附近、β 在 0 附近 → 初始状态 ≈ 恒等 → 新生儿无偏置。
```

### 6.3 学习

调制网络的权重 `W1, b1, W2_γ, W2_β` 通过**端到端梯度**更新——来自 L4 模式匹配的损失反传。

但学习率极低（`η_film = 0.0001`），因为「个性」是缓慢沉淀的，不是一两次经历就改变的。

### 6.4 Rust 结构

```rust
// 新建 context_modulation.rs

pub const CTX_DIM: usize = 8;
pub const FILM_HIDDEN: usize = 16;

pub struct ContextModulator {
    /// 上下文 → 隐藏层
    pub w1: [[f64; CTX_DIM]; FILM_HIDDEN],
    pub b1: [f64; FILM_HIDDEN],

    /// 隐藏层 → γ
    pub w2_gamma: [[f64; FILM_HIDDEN]; N_POOLS],

    /// 隐藏层 → β
    pub w2_beta: [[f64; FILM_HIDDEN]; N_POOLS],
}

impl ContextModulator {
    pub fn new() -> Self {
        // W2 全零初始化 → γ=1, β=0
    }

    /// 每 tick: 读内部状态 → 生成 γ,β → 调制特征
    pub fn modulate(
        &self,
        features: &[f64; N_POOLS],
        context: &[f64; CTX_DIM],
    ) -> [f64; N_POOLS];

    /// 慢速权重更新
    pub fn update(&mut self, grad_gamma: &[f64; N_POOLS], grad_beta: &[f64; N_POOLS]);
}
```

### 6.5 注意：γ 是乘法 = 非线性

`γ × features` 中的乘法是上下文与感知的交互——这本身就是 L2 之外的额外非线性。FiLM 层同时解决了 #3（上下文偏置）和加强了 #1（非线性能力）。一石二鸟。

---

## 7. L4 — 模式库 + 补全 + 分离

### 7.1 科学依据

**Hopfield Network（1982，2024 诺贝尔物理学奖）**
- 联想记忆：存储规则 `W = Σ pᵢ·pᵢᵀ`（赫布学习）
- 检索 = 从残缺输入迭代收敛到最近的完整存储模式 = 补全
- 经典容量 ~0.14N

**Modern Hopfield Networks（Ramsauer 2020）**
- 能量函数从二次换成指数 → 容量从 O(N) 到 O(e^N)
- **核心发现：Transformer attention = 一步 Modern Hopfield 更新**
- `retrieved = softmax(q·Kᵀ/√d) · V`

**Complementary Learning Systems（McClelland 1995）**
- 海马体：快速存储，一次学习（情节记忆）
- 新皮层：缓慢整合，多次重放（语义记忆）
- 夜间重放防灾难性遗忘

**Pattern Completion vs Separation（Yassa & Stark 2011）**
- CA3（递归侧支 ~12000/cell）：相似输入 → 收敛到同一模式（补全）
- 齿状回（2-5% 活跃率）：相似输入 → 推开成不同模式（分离）
- 只有补全 → 过度泛化；只有分离 → 无法学习

### 7.2 数学规格

#### 7.2.1 模式库

```
模式矩阵 P ∈ R^{M × D_pattern}

  M = 当前存储模式数（从 0 增长到 M_max）
  D_pattern = N_POOLS = 32（L3 输出维度）
  M_max = 2048（软上限，LRU 淘汰最旧最弱）
```

#### 7.2.2 检索（Modern Hopfield = attention）

```
q = L3 输出 ∈ R^{D_pattern}        # 当前调制后特征 = 查询

scores = P · q / √D_pattern         # [M] 个相似度分数
weights = softmax(scores / τ)        # τ = 温度（来自 L3 去甲肾上腺素等价）
retrieved = Pᵀ · weights            # [D_pattern] 检索结果 = 补全后的模式

confidence = max(weights)            # 最大权重 = 匹配信心
```

#### 7.2.3 分离门控

```
sim = cosine(q, nearest_pattern)

if sim > θ_complete:
    → 补全: 用检索结果增强/替换当前表示
    → 更新匹配模式: p ← (1-α_fast)·p + α_fast·q    # 海马快写

elif sim < θ_separate:
    → 分离: 创建新模式条目 p_new = q
    → M ← M + 1

else:
    → 模糊区: 竞争（需要注意力介入做决策）

参数:
  θ_complete  = 0.85     # 成熟后降到 0.75（区分更细差异）
  θ_separate  = 0.40     # 成熟后升到 0.55（减少冗余模式）
  α_fast      = 0.1      # 海马快写学习率
```

#### 7.2.4 慢整合（皮层学习 = CLS）

```
每 N_consolidate ticks（模拟"睡眠"）:
  1. 从模式库随机采样 batch 个模式
  2. 对相似度 > θ_merge 的模式对做 k-means 合并
  3. 合并后模式 = 加权平均（按 access_count 加权）
  4. 衰减长期未访问模式的权重

参数:
  N_consolidate = 10000   # ~100s @ 100Hz
  θ_merge       = 0.90    # 高于此的模式对合并
  decay_rate    = 0.999   # 每次整合的衰减系数
```

### 7.3 与 perturbation_graph 的关系

perturbation_graph 和 L4 模式库是**互补的两层**，不是重复：

| | perturbation_graph | L4 模式库 |
|---|---|---|
| **存什么** | 具体事件（24-D sig 的特定感知片段） | 抽象模式（L3 调制后的特征原型） |
| **怎么查** | cosine 最近邻（精确匹配） | softmax attention（软匹配 + 补全） |
| **关系** | 头等公民 `Relation`（节点间有类型化边） | 无显式关系（模式间隐式相似度） |
| **角色** | 海马体的事件层 = 情节记忆 | 皮层的抽象层 = 语义记忆 |
| **交互** | L4 检索结果 → perturbation_graph 的 mark/attention 驱动 | perturbation_graph 的 FastPath 模式 → L4 慢整合的种子 |

### 7.4 Rust 结构

```rust
// 新建 pattern_memory.rs

pub const PATTERN_DIM: usize = N_POOLS;  // 32
pub const PATTERN_CAP: usize = 2048;

pub struct PatternEntry {
    pub pattern: [f64; PATTERN_DIM],
    pub access_count: u32,
    pub last_access_t_us: u64,
    pub strength: f64,          // 沉淀强度
}

pub struct PatternMemory {
    pub entries: Vec<PatternEntry>,   // 动态增长到 PATTERN_CAP

    /// 分离/补全阈值（随发育调整）
    pub theta_complete: f64,   // 0.85 → 0.75
    pub theta_separate: f64,   // 0.40 → 0.55

    /// 检索温度（受 L3 去甲肾上腺素等价调制）
    pub temperature: f64,
}

impl PatternMemory {
    /// 每 tick: 查询 → 补全/分离/模糊 三选一
    pub fn query(
        &mut self,
        q: &[f64; PATTERN_DIM],
        t_us: u64,
    ) -> PatternQueryResult;

    /// 慢整合（每 N_consolidate ticks）
    pub fn consolidate(&mut self);

    /// 从 perturbation_graph FastPath 导入种子模式
    pub fn seed_from_fast_path(&mut self, sig: &[f64; 24]);
}

pub enum PatternQueryResult {
    Completed { pattern: [f64; PATTERN_DIM], confidence: f64 },
    Separated { new_id: usize },
    Ambiguous { top_candidates: Vec<(usize, f64)> },
}
```

---

## 8. 端到端数据流

```
每 tick (100 Hz):

1. 原始轴值 x[167] 从 viewport 读取
                │
2. L0 发育遮蔽:  x_eff = x ⊙ dev_mask(maturity)
                │
3. L1 投射+融合: z = Σ (precision_m × W_m · x_eff_m) / Σ precision_m
   L1 竞争:     a ← dynamics(a, z, w_self, w_inhib, θ)
   L1 广播:     canvas = a ⊙ ignited_mask
                │
4. L2 稀疏推断: a_sparse = ISTA(canvas, Dictionary, 5 steps)
   L2 池化:     c = energy_pool(a_sparse, groups)
   L2 归一化:   features = div_norm(c, σ)
                │
5. L3 调制:     ctx = read_intero_state()
                [γ, β] = MLP(ctx)
                modulated = γ ⊙ features + β
                │
6. L4 检索:     result = pattern_memory.query(modulated)
                match result {
                    Completed(p, conf) => {
                        // 识别到已知模式 → 可驱动意图
                        feed_intention(p, conf);
                    }
                    Separated(id) => {
                        // 新模式 → 惊奇 → 探索
                        spike_surprise();
                    }
                    Ambiguous(candidates) => {
                        // 需要更多信息 → 注意力聚焦
                        request_attention_focus(candidates);
                    }
                }
                │
7. 现有 AR-2:   predictor.step() → err → vpc → 发育时钟
                │
                L4 匹配误差 → 反传梯度 → 更新 L3 权重、L2 字典、L1 投射
```

---

## 9. 学习信号流

```
                    L4 匹配误差
                   (模式库检索残差)
                        │
          ┌─────────────┼─────────────┐
          ↓             ↓             ↓
     L3 权重更新    L2 字典更新    L1 投射更新
     (极慢 0.0001)  (慢 0.001)    (慢 0.001)
     = 个性漂移     = 特征涌现    = 模态对齐

                    AR-2 预测误差
                   (现有 viewport err)
                        │
          ┌─────────────┼─────────────┐
          ↓             ↓             ↓
     L1 精度权重    AR-2 系数     VPC cross-layer
     (模态可靠性)   (不变)        (不变)
```

两套学习信号独立：
- **AR-2 误差** 驱动低层预测质量 + 模态精度估计（已存在）
- **L4 匹配误差** 驱动中间层特征质量（新增）

---

## 10. 实施计划

### 10.1 依赖顺序

```
L0（发育降采样）──→ L1（共享画布）──→ L2（非线性特征）──→ L4（模式库）
                                         │
                                         └──→ L3（FiLM 调制）
```

L0 和 L1 是地基——没有画布，后面的层没有地方放。
L3 依赖 L2 的输出存在，但可以晚于 L4 实施（L4 可以先不调制直接吃 L2 输出）。

### 10.2 分阶段

| 阶段 | 内容 | 新文件 | 估计复杂度 |
|------|------|--------|-----------|
| **Phase 1** | L0 发育降采样 + L1 共享画布（无学习，固定随机投射） | `developmental_gate.rs`, `shared_canvas.rs` | 中 |
| **Phase 2** | L2 稀疏编码 + 池化 + 归一化 | `feature_extraction.rs` | 中高 |
| **Phase 3** | L4 模式库（Modern Hopfield 检索 + 分离门控） | `pattern_memory.rs` | 中 |
| **Phase 4** | L3 FiLM 调制 | `context_modulation.rs` | 低 |
| **Phase 5** | 端到端梯度 + L1 投射学习 + L2 字典学习 | 修改以上各文件 | 高 |
| **Phase 6** | 注意力统一（现有三套 → 画布点火） | 修改 `viewport.rs`, `active_surface.rs`, `prediction_pressure_field.rs` | 高（接线重构） |

### 10.3 验证标准

| 阶段 | 通过标准 |
|------|---------|
| Phase 1 | 画布激活模式对输入变化有响应；发育遮蔽正确钳零 |
| Phase 2 | 字典列在训练后呈现可解释的特征模式（非随机噪声） |
| Phase 3 | 给部分线索能检索到正确的完整模式（补全率 > 80%，30% 遮挡下） |
| Phase 4 | 不同内部状态下，同一输入的 L4 检索结果不同 |
| Phase 5 | L2 字典随训练时间自组织（特征可视化显示有结构） |
| Phase 6 | 旧的 GlobalAttentionState 字段全部可从画布状态派生 |

---

## 11. 参数总表

| 参数 | 值 | 所在层 | 理由 |
|------|-----|--------|------|
| `CANVAS_DIM` | 64 | L1 | 6 模态汇聚空间，64 够覆盖主要变异 |
| `N_FEATURES` | 128 | L2 | 过完备字典（> CANVAS_DIM）促进稀疏性 |
| `N_POOLS` | 32 | L2 | 池化后特征数 = 模式维度 |
| `CTX_DIM` | 8 | L3 | 8 个 INTERO + 发育状态 |
| `FILM_HIDDEN` | 16 | L3 | 调制网络隐藏层（小 = 防过拟合） |
| `PATTERN_CAP` | 2048 | L4 | 模式库容量上限 |
| `w_self` | 1.2 | L1 | 自激强度（> 1 才能点火） |
| `w_inhib` | 0.15 | L1 | 侧抑制（太大 → 只有一个维度能活） |
| `θ` (theta) | 0.3 | L1 | 点火阈值 |
| `λ` (lambda) | 0.1 | L2 | 稀疏惩罚（太大 → 全零，太小 → 不稀疏） |
| `σ²` | 0.01 | L2 | 除法归一化半饱和常数 |
| `θ_complete` | 0.85→0.75 | L4 | 补全阈值（随发育降低 = 更细区分） |
| `θ_separate` | 0.40→0.55 | L4 | 分离阈值（随发育升高 = 减少冗余） |
| `α_fast` | 0.1 | L4 | 海马快写学习率 |
| `η_film` | 0.0001 | L3 | FiLM 权重学习率（极慢 = 个性沉淀） |
| `N_consolidate` | 10000 | L4 | 慢整合间隔 ~100s |

---

## 12. 研究引用

| 层 | 关键论文 | 年份 | 核心贡献 |
|----|---------|------|---------|
| L0 | Elman, "Starting Small" | 1993 | 简化输入提升学习效果 |
| L0 | Turkewitz & Kenny, 成熟性约束 | 1982 | 早期感官受限是 feature |
| L1 | Baars, Global Workspace Theory | 1988 | 意识 = 全局广播 |
| L1 | Dehaene & Changeux, GNW ignition | 2011 | 点火 = 双稳态跳变 |
| L1 | Damasio, Convergence-Divergence Zones | 2010 | 层级式多模态汇聚 |
| L2 | Hubel & Wiesel, 简单/复杂细胞 | 1962 | 线性滤波 + 非线性池化 |
| L2 | Olshausen & Field, 稀疏编码 | 1996 | 稀疏约束自动涌现 V1 特征 |
| L2 | Carandini & Heeger, 除法归一化 | 2012 | "大脑标准计算" |
| L2 | Rao & Ballard, 预测编码 | 1999 | 层级预测误差传播 |
| L3 | Salinas & Abbott, Gain modulation | 2001 | 乘法增益调制 |
| L3 | Perez et al., FiLM | 2018 | Feature-wise Linear Modulation |
| L4 | Hopfield, 联想记忆 | 1982 | 吸引子网络 = 补全 |
| L4 | Ramsauer et al., Modern Hopfield | 2020 | Attention = Hopfield 更新 |
| L4 | McClelland, CLS | 1995 | 双速学习防灾难性遗忘 |
| L4 | Yassa & Stark, 补全 vs 分离 | 2011 | CA3 补全 + 齿状回分离 |
| L4 | Vincent, Denoising Autoencoders | 2008 | 破损输入训练 = 学补全 |

---

## 13. 风险登记

| 风险 | 影响 | 缓解 |
|------|------|------|
| **自举死锁** | L2 字典没数据学不出特征 → L4 没特征建不了模式 → 系统空转 | L0 保证最少 32 轴输入；字典初始化为 PCA 方向（非纯随机）；Phase 1 先用固定投射验证数据流 |
| **计算开销** | 5 步 ISTA + 画布竞争 + attention 检索 → 100Hz 可能来不及 | ISTA 步数可降到 3；画布竞争一步欧拉足够；attention 只对 top-K 模式算（非全库） |
| **稀疏编码收敛** | 字典可能坍缩到少数高频使用的基 | 加字典列归一化（每列 L2=1）+ 未使用列随机重置 |
| **灾难性遗忘** | 新模式覆写旧模式 | CLS 双速学习 + 慢整合间隔 |
| **注意力统一（Phase 6）** | 改动现有三套系统风险大 | 最后做；先让 L1-L4 与现有系统并行跑，确认稳定后再切换 |

---

*文档结束。业主过目后决定是否动工、从哪个 Phase 开始。*
