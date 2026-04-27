# Tier 0 元素 → 当前代码落点地图

> 生成时间：2026-04-22
> 基于：`tier0-base-elements.md` v0.8
> 代码基线：`bionic_v0/` snapshot version 29
> 规则：只映射、不发明、不塞高层、不改代码

---

## 图例

- **✅ 完整落点**：元素在代码中有对应存储/计算结构
- **⚠️ 部分落点**：子集、近似、或工程壳
- **❌ 无落点**：元素在当前代码中不存在

---

## 一、落在 Viewport（全局层）

> `Viewport` struct：`viewport.rs:891-982`
> 这些元素存在于 viewport 全局状态中，不属于任何特定对象槽位。

### 1.1 `time_t` — 全局时间挂载 ✅

| 代码位置 | 说明 |
|----------|------|
| `Viewport::clock_us: AtomicU64` | 全局当前时间（µs） |
| `Viewport::start_us: AtomicU64` | 系统启动时间 |
| `Viewport::master_tick: AtomicU64` | 1kHz 主时钟节拍计数 |
| `clock.rs::now_us()` | 墙钟微秒读取函数 |

**时间曲线载体**：
- `AxisRing`（`viewport.rs:57-105`）：`buf: [Sample; HISTORY_CAP]`，`Sample { t_us, value, valid }`
- `HISTORY_CAP = 1024`（`lib.rs:14`）
- 每个轴一个 `AxisRing`，通过 `Viewport::observed/predicted/err_stream[axis]` 访问

**当前承载的轴**：
- `observed[0..AXIS_COUNT]` → `vib_band_1..4` 的观察历史
- `predicted[0..AXIS_COUNT]` → `vib_band_1..4` 的预测历史
- `err_stream[0..AXIS_COUNT]` → `vib_band_1..4` 的误差历史

### 1.2 `vib_band_1..4` — 振动频带 ✅

| 代码位置 | 说明 |
|----------|------|
| `AXIS_COUNT = VIB_BANDS = 4`（`lib.rs:16-17`） | 轴数定义 |
| `VIB_BAND_CUTOFFS: [500.0, 2000.0, 5000.0]`（`lib.rs:21`） | 频带分界 |
| `Viewport::observed/predicted/err_stream[axis]` | 全局轴级时间曲线 |
| `Viewport::predictors[axis]: PredictorState` | 全局 AR-2 预测器（`predictor.rs`） |
| `Viewport::hl: HighLevelState` | 全局高层 MSE 跟踪（`highlevel.rs`） |
| `AmbientChannel::vib`（`viewport.rs:186`） | 环境通道：`event_energy`, `surprise`, `last_event_t_us`, `candidate_flag` |

### 1.3 环境通道（非 tier 0 轴，感知事件检测层）⚠️

| 代码位置 | 说明 |
|----------|------|
| `AmbientChannel`（`viewport.rs:184-199`） | 包含 `vib`, `phot`, `mech` 三个子通道 |
| `nomination_seq: AtomicU64` | 全局候选序列号，槽位分配的统一信号源 |
| `AmbientModalityState`（`viewport.rs:151-156`） | `event_energy`, `surprise`, `last_event_t_us`, `candidate_flag` |

> 注意：环境通道不是 tier 0 元素。它是感知 ingress 的事件检测层，在 tier 0 轴值写入槽位之前运行。

---

## 二、落在对象槽位（ObjectSlot）

> `ObjectSlot` struct：`viewport.rs:821-848`
> 固定 `N_OBJECT_SLOTS = 4`（`lib.rs:24`）
> 每个槽位是一个多模态容器，按模态分枝，不按名称索引。

### 2.1 `vib_band_1..4` 的槽位级片段 ✅

| ObjectSlot 字段 | 对应 tier 0 | 说明 |
|-----------------|-------------|------|
| `vib: Option<BandFamilyState>` | `vib_band_1..4` | 槽位级频带片段：`observed[n]`, `predicted[n]`, `err[n]`, `precision[n]` |

`BandFamilyState`（`viewport.rs:217-223`）：
```
n_bands: u16
observed: Vec<f64>     // 当前周期观察值
predicted: Vec<f64>    // 当前周期预测值
err: Vec<f64>          // 观察 - 预测
precision: Vec<f64>    // 精度/置信度
```

### 2.2 `pos_x`, `pos_y`, `span_x`, `span_y` — 2D 空间子集 ⚠️

| ObjectSlot 字段 | 对应 tier 0 | 说明 |
|-----------------|-------------|------|
| `spatial: Option<SpatialState>` | `pos_x`, `pos_y`, `span_x`, `span_y` | 仅 2D 归一化屏幕坐标 `[0,1]²` |

`SpatialState`（`viewport.rs:248-257`）：
```
pos_x: f64        // → tier 0 pos_x（2D 归一化，非物理空间）
pos_y: f64        // → tier 0 pos_y（2D 归一化）
span_x: f64       // → tier 0 span_x（2D 归一化）
span_y: f64       // → tier 0 span_y（2D 归一化）
precision: f64    // 位置估计置信度（非 tier 0 轴）
last_update_t_us: u64
valid: bool       // 有效性标记（非 tier 0 轴）
```

> 代码注释原文："Coordinates and spans are normalized f64 in [0,1]; the substrate assigns no coordinate-system name, depth, or eye-specific channel."

### 2.3 `light_band_1..n` 的近似 — PHOT 运动能量 ⚠️

| ObjectSlot 字段 | 对应 tier 0 | 说明 |
|-----------------|-------------|------|
| `phot: Option<BandFamilyState>` | `light_band` 的近似 | 存储的是运动能量，不是光谱分解 |

> PHOT 适配器（`phot_camera.rs`）提供 64×48 灰度帧，`MotionExtractor` 提取运动区域。频带值来自运动能量，不是 tier 0 光谱分解。`PHOT_BANDS = 4`（`lib.rs:119`）。

### 2.4 预留但未激活

| ObjectSlot 字段 | 对应 tier 0 | 状态 |
|-----------------|-------------|------|
| `mech: Option<BandFamilyState>` | 力学响应频带（预留） | ❌ 未激活，无 adapter 写入 |

### 2.5 槽位级 tier 1 结构（非 tier 0，记录在此避免混淆）

| ObjectSlot 字段 | 层级 | 说明 |
|-----------------|------|------|
| `fusion: Option<CrossModalFusionState>` | tier 1 | 跨模态融合：时间对齐、置信度、权重 |
| `high_layer: Option<ObjectHighLayerState>` | tier 1 | 高层特征输出 |
| `phot_pred: Option<PhotSlotPredictor>` | tier 1 | PHOT 槽位预测器（ephemeral，不 snapshot） |
| `hl_pred: Option<ObjectHighLayerPredictor>` | tier 1 | 高层预测器（ephemeral，不 snapshot） |
| `attention_share: f64` | tier 1+ | 注意力分配 |
| `precision_bias: f64` | tier 1+ | 精度偏置建议 |

### 2.6 槽位元数据（非 tier 0 轴）

| ObjectSlot 字段 | 说明 |
|-----------------|------|
| `active: bool` | 槽位是否活跃 |
| `last_update_t_us: u64` | 最后更新时间 |
| `source_modality: u8` | 模态位图（`MOD_VIB | MOD_PHOT | MOD_MECH`） |
| `nomination_seq: u64` | 分配时的全局序列号 |

---

## 三、落在关系（GlobalRelationState）

> `GlobalRelationState`：`viewport.rs:435-440`
> tier 0 v0.8 定义：**关系不是存储实体，而是对多对象 tier 0 状态的查询时投影。**
> 当前代码中的关系层 **不是 tier 0 轴投影**，而是基于槽位级高层特征的 EMA 关系分数。

### 3.1 关系边结构

`RelationEdgeState`（`viewport.rs:410-430`）：
```
src_slot_id: u8
dst_slot_id: u8
active: bool
co_change_score: f64      // [0,1] EMA 相似度：1 - |bias_src - bias_dst|
lead_lag_score: f64       // [-1,1] EMA 时间先后：(t_dst - t_src) / CROSS_MODAL_WINDOW_US
predictive_score: f64     // [0,1] EMA 因果前导：src error-spike before dst error-spike
relation_confidence: f64  // [0,1] EMA 置信度：4 * share_src * share_dst
last_update_t_us: u64
pred: RelationEdgePredictionState  // Phase 5E 预测器输出
```

### 3.2 关系视角映射

| tier 0 关系视角 | 代码落点 | 状态 | 差距说明 |
|-----------------|----------|------|----------|
| **时间**（先后、共时） | `lead_lag_score` | ⚠️ | 基于槽位事件时间差，不是 `time_t` 曲线投影 |
| **动态**（趋同/趋异） | `co_change_score` | ⚠️ | 基于 `attention_bias`（tier 1），不是 tier 0 轴曲线的 d/dt |
| **动态**（因果前导） | `predictive_score` | ⚠️ | 基于 prediction error 的时间交叉，不是 tier 0 轴 |
| **空间**（距离、方向） | — | ❌ | 关系计算不读取 `SpatialState` |
| **物理** | — | ❌ | 无对应 tier 0 轴 |
| **光学** | — | ❌ | 无对应 tier 0 轴 |
| **化学** | — | ❌ | 无对应 tier 0 轴 |
| **认知** | — | ❌ | 无 ∫presence·dt 累计观察 |

### 3.3 关系预测器（Phase 5E）

`RelationEdgePredState`（`viewport.rs:382-391`）：
```
ema_co: f64        // co_change 的 AR-1 EMA
ema_lag: f64       // lead_lag 的 AR-1 EMA
ema_pred: f64      // predictive 的 AR-1 EMA
ema_conf: f64      // confidence 的 AR-1 EMA
ema_mse: f64       // 预测误差 MSE
warmup: u32        // 预热计数
```

`RelationPredBank`（`viewport.rs:395-405`）：
- `states: [RelationEdgePredState; N_REL_EDGES]`
- `N_REL_EDGES = 4 × 3 = 12`（`lib.rs:236`）
- Ephemeral，不 snapshot

### 3.4 关系分数预测输出（snapshot 可见）

`RelationEdgePredictionState`（`viewport.rs:361-377`）：
```
predicted_co_change: f64
predicted_lead_lag: f64
predicted_predictive: f64
predicted_confidence: f64
relation_err: f64
relation_precision: f64
last_predict_t_us: u64
```

---

## 四、落在写出历元（ActionOutput）

> `action_output.rs`
> 实现 spec：`docs/superpowers/specs/2026-04-21-action-output-substrate-minimal-contract-design.md`

### 4.1 写出历元记录 ✅

`WriteEpochRecord`（`action_output.rs:68-78`）：
```
write_seq: u64              // 单调配对句柄
channel_kind: OutputChannelKind  // 当前只有 GenericWrite
start_t_us: u64             // → tier 0 time_t（写出开始时间）
last_update_t_us: u64       // → tier 0 time_t（最后更新时间）
last_phase: ActionOutputPhase   // Idle/Active/Decaying
```

### 4.2 写出通道追踪 ✅

`ActionOutputWriteTrace`（`action_output.rs:46-60`）：
```
channel_kind: OutputChannelKind
phase: ActionOutputPhase
current_magnitude: f64      // [0,1] 当前写出强度
peak_magnitude: f64         // 历史峰值
start_t_us: u64             // → tier 0 time_t
last_update_t_us: u64       // → tier 0 time_t
last_transition_t_us: u64   // → tier 0 time_t（相位切换时间）
reinforce_count: u32        // 强化计数
decay_count: u32            // 衰减计数
current_write_seq: u64      // 当前写序列号
```

### 4.3 写出状态聚合 ✅

`ActionOutputState`（`action_output.rs:80-89`）：
```
n_channels: u32
channels: Vec<ActionOutputWriteTrace>
write_epoch_history: Vec<WriteEpochRecord>  // 追加型历史
n_write_epochs: u32
```

- `WRITE_EPOCH_HISTORY_CAP = 128`（`lib.rs:150`）
- FIFO 淘汰，按追加顺序

### 4.4 写出生命周期

`ActionOutputFeedback`（`action_output.rs:30-36`）：`Begin` → `Adjust` → `Decay` → `Stop`

`ActionOutputPhase`（`action_output.rs:22-28`）：`Idle` → `Active` → `Decaying`

### 4.5 差距

| 差距 | 说明 |
|------|------|
| 通道类型单一 | `OutputChannelKind` 只有 `GenericWrite`，无物理域区分 |
| 无 tier 0 轴关联 | 写出历元不携带 tier 0 轴签名，只记录时间和强度 |

---

## 五、落在后果窗口（ActionConsequence）

> `action_consequence.rs`
> 实现 spec：`docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md`

### 5.1 后果窗口 ✅

`ActionConsequenceWindow`（`action_consequence.rs:59-75`）：
```
window_id: u64              // 稳定单调 ID（不重用）
modality_kind: ConsequenceModalityKind  // 当前只有 GenericIngress
start_t_us: u64             // → tier 0 time_t（窗口开始）
end_t_us: u64               // → tier 0 time_t（窗口结束）
magnitude: f64              // 后果强度
phase: ConsequenceWindowPhase  // Open / Closed
last_update_t_us: u64       // → tier 0 time_t
```

生命周期 API：
- `begin_action_consequence_window()` → Open
- `extend_action_consequence_window()` → 推进 end_t_us
- `close_action_consequence_window()` → Closed

### 5.2 写-后果配对 ✅

`ActionConsequencePair`（`action_consequence.rs:77-86`）：
```
write_seq: u64              // 引用 WriteEpochRecord::write_seq
window_id: u64              // 引用 ActionConsequenceWindow::window_id
delay_us: u64               // 写出到后果的时间间隔
status: ConsequencePairStatus   // Pending / Paired
pair_t_us: u64              // → tier 0 time_t（配对时间）
```

### 5.3 后果状态聚合 ✅

`ActionConsequenceState`（`action_consequence.rs:88-94`）：
```
n_windows: u32
windows: Vec<ActionConsequenceWindow>
n_pairs: u32
pairs: Vec<ActionConsequencePair>
```

- `CONSEQUENCE_WINDOW_CAP = 128`（`lib.rs:157`）
- `PAIR_HISTORY_CAP = 256`（`lib.rs:164`）
- FIFO 淘汰

### 5.4 自动配对

- `accumulate_ingress_change()` — 记录 ingress 变化到窗口
- `try_auto_pair_latest_epoch_to_window()` — 按 `TEMPORAL_PAIR_MAX_DELAY_US = 2s` 自动配对
- `pair_action_consequence()` — 显式配对

---

## 六、无落点

> 以下 tier 0 元素在当前代码中 **没有任何存储、计算、或工程壳**。

### 6.1 STRUCT — 空间状态（缺失部分）

| tier 0 轴 | 说明 |
|-----------|------|
| `pos_z` | 深度坐标 |
| `span_z` | 深度展开 |
| `dir_x`, `dir_y`, `dir_z` | 对象朝向（单位向量） |
| `chirality` | 手性偏置 [-1, 1] |
| `aniso_dir_x`, `aniso_dir_y`, `aniso_dir_z` | 各向异性主方向 |
| `aniso_ratio` | 各向异性强度比 [1, +∞) |

### 6.2 INRT — 惯性

| tier 0 轴 | 说明 |
|-----------|------|
| `mass` | 惯性质量 |

### 6.3 MECH — 力学响应

| tier 0 轴 | 说明 |
|-----------|------|
| `compress_res` | 抗压缩 |
| `shear_res` | 抗剪切 |
| `fracture_res` | 抗断裂 |
| `recover_res` | 弹性恢复 [0, 1] |
| `surface_drag` | 表面滑动阻滞 |
| `adhesion` | 表面对外物黏附 |
| `cohesion` | 自聚/内聚 |

### 6.4 PHOT-IN — 光学入射（模态部分）

| tier 0 轴 | 说明 |
|-----------|------|
| `reflect` | 反射比 [0, 1] |
| `transmit` | 透射比 [0, 1] |
| `scatter` | 散射比 [0, 1] |
| `polar_resp` | 偏振选择性 [0, 1] |
| `polar_dir` | 选择性偏振轴角度 [0, π) |

### 6.5 PHOT-OUT — 光学发射

| tier 0 轴 | 说明 |
|-----------|------|
| `emit_band_1..n` | 自身发射光谱 |

### 6.6 THRM — 热学响应

| tier 0 轴 | 说明 |
|-----------|------|
| `thermal_flow` | 热传导系数 |
| `thermal_cap` | 热容系数 |
| `thermal_state` | 当前温度 |

### 6.7 ELEC — 电磁响应（电）

| tier 0 轴 | 说明 |
|-----------|------|
| `electric_flow` | 电导系数 |
| `charge_bias` | 净电荷状态 |
| `electric_polariz` | 静态介电极化率 |

### 6.8 MAGN — 电磁响应（磁）

| tier 0 轴 | 说明 |
|-----------|------|
| `mag_response` | 磁化率 |
| `mag_retain` | 残磁保持 [0, 1] |

### 6.9 PART-IN — 粒子入射

| tier 0 轴 | 说明 |
|-----------|------|
| `particle_in_band_1..n` | 按粒子类型的吸收/屏蔽响应 |

### 6.10 PART-OUT — 粒子发射

| tier 0 轴 | 说明 |
|-----------|------|
| `particle_emit_band_1..n` | 按粒子类型的发射强度 |

### 6.11 VIB — 振动响应

| tier 0 轴 | 说明 |
|-----------|------|
| `vib_band_1..n` | ⚠️ 已有落点（见 §1.2, §2.1），但当前 n=4 是最小实例 |

### 6.12 CHEM-IN — 化学入射

| tier 0 轴 | 说明 |
|-----------|------|
| `chem_band_1..n` | 按化学通道的入射响应 |

### 6.13 CHEM-OUT — 化学发射

| tier 0 轴 | 说明 |
|-----------|------|
| `chem_emit_band_1..n` | 按化学通道的主动发射 |

### 6.14 STRUCT-ORDER — 结构有序度

| tier 0 轴 | 说明 |
|-----------|------|
| `struct_order_pos` | 平移位置有序度 [0, 1] |
| `struct_order_orient` | 方向/取向有序度 [0, 1] |
| `struct_order_period` | 周期性强度 [0, 1] |

### 6.15 MECH 穿透 — 选择性穿透

| tier 0 轴 | 说明 |
|-----------|------|
| `permeate_band_1..n` | 按化学通道的穿透阻力 |

---

## 七、汇总统计

### 按落点分类

| 落点类别 | 元素 | 占比 |
|----------|------|------|
| ✅ Viewport 全局层 | `time_t`, `vib_band_1..4` | 2 轴 |
| ✅ 对象槽位层 | `vib_band_1..4`（槽位片段）, `pos_x/y`, `span_x/y`（2D 子集）, `light_band` 近似 | 4 轴 + 2 近似 |
| ⚠️ 关系层 | 时间 lead-lag、动态 co-change、因果 predictive | 3 个分数（非 tier 0 投影） |
| ✅ 写出历元层 | `time_t`（通过 `*_t_us`） | time_t 复用 |
| ✅ 后果窗口层 | `time_t`（通过 `*_t_us`） | time_t 复用 |
| ❌ 无落点 | 其余 27 个固定轴 + 5 组频带轴 | ~85% |

### 按 tier 0 族统计

| 族 | 轴数 | 有落点 | 部分落点 | 无落点 |
|----|------|--------|----------|--------|
| TIME | 1 | `time_t` | — | — |
| STRUCT | 10 | — | `pos_x/y`, `span_x/y` | `pos_z`, `span_z`, `dir_*`, `chirality`, `aniso_*` |
| INRT | 1 | — | — | `mass` |
| MECH | 8 | — | — | 全部 |
| PHOT-IN | 8 | — | `light_band` 近似 | `reflect`, `transmit`, `scatter`, `polar_*` |
| PHOT-OUT | 1 | — | — | `emit_band` |
| THRM | 3 | — | — | 全部 |
| ELEC | 3 | — | — | 全部 |
| MAGN | 2 | — | — | 全部 |
| PART-IN | 1 | — | — | `particle_in_band` |
| PART-OUT | 1 | — | — | `particle_emit_band` |
| VIB | 1 | `vib_band` | — | — |
| CHEM-IN | 1 | — | — | `chem_band` |
| CHEM-OUT | 1 | — | — | `chem_emit_band` |
| STRUCT-ORDER | 3 | — | — | 全部 |
| MECH 穿透 | 1 | — | — | `permeate_band` |
| **合计** | **~46** | **2** | **~5** | **~39** |

### 已就位的基础设施（可扩展载体）

| 基础设施 | 位置 | 可扩展为 |
|----------|------|----------|
| `AxisRing` | `viewport.rs:57-105` | 任意 tier 0 轴的时间曲线 |
| `BandFamilyState` | `viewport.rs:217-223` | 任意频带族（light/emit/part/chem/vib/permeate） |
| `ObjectSlot` 多模态架构 | `viewport.rs:821-848` | 新增模态子树 |
| `SpatialState` | `viewport.rs:248-257` | 扩展为 3D（加 pos_z/span_z） |
| `RelationEdgeState` | `viewport.rs:410-430` | 扩展为 tier 0 轴投影 |
| Snapshot v29 序列化 | `snapshot.rs` | 版本门控支持新字段 |
| Observer | `bin/observer.rs` | 渲染新轴 |
