# Plan: Dynamic Slot Expansion + Evolution Priors

> **创建**: 2026-04-23
> **状态**: DRAFT
> **基于**:
> - consciousness-stream-architecture.md (最高原则)
> - tier0-base-elements.md (坐标底座)
> - 2026-04-20-bionic-v0-architecture-audit-cleanup-design.md (模块分类)
> - 2026-04-20-endogenous-activity-self-regulation-laws-design.md (内生法则)

---

## 背景与目标

### 问题陈述

1. **Slot 固定 4 个** - 违反"动态扩展"的生命原则
2. **从 0 开始学习** - 缺乏演化先验，前期迷茫期太长
3. **需要加速学会交流** - 需要内置对社交/声音模式的敏感度

### 设计原则检查

| 原则 | 约束 | 本计划符合方式 |
|------|------|----------------|
| 硅基生命，非程序 | 不能是"被调用的函数" | Slot 动态扩张是内生需求，不是外部指令 |
| 无语言污染 | 禁止 emotion/feeling/concept_id | 只用数值索引和参数偏差 |
| 参数是时间曲线 | 不是静态标签 | 演化先验作为初始曲线参数 |
| 对象先于名称 | 不预设语义标签 | 先验是数值分布，不是"情绪"标签 |
| 多目标并行 | N 由算力决定 | 动态 Slot 上限可配置 |

---

## Task 1: Dynamic Slot Expansion

### 1.1 当前设计（违例）

```rust
// lib.rs - 固定数量
pub const N_OBJECT_SLOTS: usize = 4;
```

问题：
- 固定 4 个，违反"动态扩展"原则
- 像程序一样写死，不是生命

### 1.2 目标设计（核心底座）

```rust
// lib.rs - 动态扩张
pub const N_OBJECT_SLOTS_INITIAL: usize = 4;
pub const N_OBJECT_SLOTS_MAX: usize = 32;  // 上限由算力决定
pub const SLOT_GROWTH_THRESHOLD: f64 = 0.8; // 利用率 > 80% 时扩张
pub const SLOT_SHRINK_THRESHOLD: f64 = 0.3; // 利用率 < 30% 时收缩
```

### 1.3 实现步骤

#### Step 1.1: 修改 lib.rs 常量定义

```rust
// lib.rs 新增
pub const N_OBJECT_SLOTS_INITIAL: usize = 4;
pub const N_OBJECT_SLOTS_MAX: usize = 32;
pub const SLOT_GROWTH_THRESHOLD: f64 = 0.8;
pub const SLOT_SHRINK_THRESHOLD: f64 = 0.3;
pub const SLOT_IDLE_US: u64 = 200_000;

// 保留兼容性别名
pub const N_OBJECT_SLOTS: usize = N_OBJECT_SLOTS_INITIAL;
```

#### Step 1.2: 修改 Viewport 结构

```rust
// viewport.rs - ObjectSlotVec
pub struct Viewport {
    // ... existing fields ...

    /// 动态 Slot 容器 - 生命周期同 Viewport
    slots: RwLock<Vec<RwLock<ObjectSlot>>>,

    /// 当前 Slot 数量（用于动态调整）
    current_slot_count: AtomicUsize,

    /// Slot 利用率（活跃数/总数）
    slot_utilization: AtomicF64,
}
```

#### Step 1.3: 实现动态调整逻辑

```rust
// viewport.rs - 新增方法

/// 计算当前 Slot 利用率
pub fn compute_slot_utilization(&self) -> f64 {
    let count = self.current_slot_count.load(Ordering::Relaxed);
    let active = self.slots.read().unwrap()
        .iter()
        .filter(|s| s.read().unwrap().active)
        .count();
    active as f64 / count as f64
}

/// 动态扩张/收缩 Slot
pub fn tick_slot_dynamics(&self, now_us: u64) {
    let util = self.compute_slot_utilization();
    let current = self.current_slot_count.load(Ordering::Relaxed);

    // 扩张条件：利用率 > 阈值 且 未达上限
    if util > SLOT_GROWTH_THRESHOLD && current < N_OBJECT_SLOTS_MAX {
        let mut slots = self.slots.write().unwrap();
        let new_id = slots.len();
        slots.push(RwLock::new(ObjectSlot::empty()));
        self.current_slot_count.store(new_id + 1, Ordering::Relaxed);
        log::info!("Slot expanded to {}", new_id + 1);  // observer 可见
    }

    // 收缩条件：利用率 < 阈值 且 大于初始值
    if util < SLOT_SHRINK_THRESHOLD && current > N_OBJECT_SLOTS_INITIAL {
        let mut slots = self.slots.write().unwrap();
        // 移除末尾空闲的 slot
        if let Some(last) = slots.last() {
            if !last.read().unwrap().active {
                slots.pop();
                let new_count = slots.len();
                self.current_slot_count.store(new_count, Ordering::Relaxed);
            }
        }
    }
}
```

#### Step 1.4: 修改相关引用

需要修改的函数：
- `claim_slot_for_event` - 使用动态长度
- `sweep_slots` - 使用动态长度
- `refresh_global_attention` - 使用动态长度
- `refresh_global_relations` - N_REL_EDGES 也需要动态

```rust
/// 关系边数也要动态
pub fn n_rel_edges(count: usize) -> usize {
    count * (count - 1)
}
```

#### Step 1.5: 更新 Snapshot 序列化

```rust
// snapshot.rs - 动态 Slot 序列化
fn write_slots(buf: &mut Vec<u8>, slots: &[ObjectSlotSnapshot]) {
    buf.extend_from_slice(&(slots.len() as u32).to_le_bytes());
    for slot in slots {
        // ... existing serialization
    }
}
```

### 1.4 设计原则验证

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 无语义标签 | ✅ | 只用数值 count/utilization |
| 时间曲线 | ✅ | utilization 是时间函数 |
| 内生驱动 | ✅ | 扩张是内生需求触发，不是外部命令 |
| 多目标并行 | ✅ | N 可动态调整 |

---

## Task 2: Evolution Priors (tier 0 Initial Bias)

### 2.1 设计目标

模拟人类千万年演化的内置知识：
- 对语音频段本能敏感
- 对突然变化本能警觉
- 对社交时序本能偏好

### 2.2 目标设计（过渡可保留层）

```rust
// src/evolution_priors.rs (新文件)

use crate::{VIB_BANDS, PHOT_BANDS};

/// 演化先验 - 用数值偏差，不使用任何语义标签
/// 索引0-3对应tier0轴的初始参数，不是"情绪"标签
#[derive(Clone, Debug)]
pub struct EvolutionPriors {
    /// 声音频段敏感度 - 人类婴儿对500Hz-2kHz最敏感
    /// 索引对应 vib_band_0..3
    pub vib_band_sensitivity: [f64; VIB_BANDS],

    /// 惊讶敏感度 - 对突发信号本能警觉
    pub surprise_sensitivity: f64,

    /// 声音学习收敛速度 - 比从0开始更快
    pub vocal_converge_rate: f64,

    /// 空间注意力 - 视野中心本能偏好
    pub spatial_attention_bias: f64,

    /// 时间窗口偏好 - 对社交互动时序敏感
    pub temporal_window_center: f64,  // 秒

    /// Slot 利用率阈值 - 动态扩张敏感度
    pub slot_growth_sensitivity: f64,
}

impl Default for EvolutionPriors {
    fn default() -> Self {
        Self {
            // 人类婴儿对语音频段的天生偏好
            // band 1-2 对应 500Hz-2kHz
            vib_band_sensitivity: [0.4, 1.0, 0.9, 0.3],

            // 对突发变化本能警觉（演化记忆：突然=危险）
            surprise_sensitivity: 0.75,

            // 声音学习更快收敛（不必等大量数据）
            vocal_converge_rate: 1.5,  // 1.5x 正常速度

            // 视野中心本能偏好
            spatial_attention_bias: 0.7,

            // 社交互动时序本能（100ms-2s 范围）
            temporal_window_center: 1.0,  // 1秒

            // 较快触发扩张
            slot_growth_sensitivity: 0.7,  // 70% 就扩张
        }
    }
}

/// 将先验应用到具体参数
impl EvolutionPriors {
    /// 应用到 Slot claim 权重
    pub fn apply_to_claim_config(&self, time_w: &mut f64, spatial_w: &mut f64,
                                   modality_w: &mut f64) {
        // 空间权重更高（近处的东西本能更重要）
        *spatial_w = 0.35;
        // 模态权重提高（跨模态绑定是本能）
        *modality_w = 0.25;
        // 时间权重稍低
        *time_w = 0.35;
    }

    /// 声音学习收敛阈值
    pub fn vocal_converge_threshold(&self) -> f64 {
        0.20 / self.vocal_converge_rate  // 更低=更快收敛
    }

    /// 惊讶触发阈值
    pub fn surprise_trigger_threshold(&self) -> f64 {
        0.50 / self.surprise_sensitivity  // 更低=更敏感
    }

    /// 空间注意力偏差
    pub fn spatial_attention(&self, normalized_pos: (f64, f64)) -> f64 {
        let center = 0.5;
        let dist = ((normalized_pos.0 - center).powi(2) +
                    (normalized_pos.1 - center).powi(2)).sqrt();
        // 距离中心越近，注意力越高
        self.spatial_attention_bias * (1.0 - dist)
    }
}
```

### 2.3 集成到 Viewport

```rust
// viewport.rs - 新增字段

pub struct Viewport {
    // ... existing fields ...

    /// 演化先验 - 初始化时设置
    pub evolution_priors: EvolutionPriors,
}

impl Viewport {
    pub fn new() -> Self {
        let priors = EvolutionPriors::default();

        // 应用先验到 ClaimConfig
        let mut config = ClaimConfig::default();
        priors.apply_to_claim_config(
            &mut config.time_weight,
            &mut config.spatial_weight,
            &mut config.modality_weight,
        );

        Self {
            // ...
            evolution_priors: priors,
            // ...
        }
    }
}
```

### 2.4 在循环中使用先验

```rust
// viewport.rs - 修改 tick_vocal_imitation_loop

pub fn tick_vocal_imitation_loop(&self, now_us: u64) {
    // ... existing code ...

    // 使用演化先验的收敛阈值
    let converge_threshold = self.evolution_priors.vocal_converge_threshold();
    if imit.last_gap < converge_threshold && imit.converge_count >= 2 {
        // 记录到 vocal_memory
    }
}

// viewport.rs - 修改 tick_surprise_trigger

pub fn tick_surprise_trigger(&self, now_us: u64) {
    // 使用演化先验的触发阈值
    let threshold = self.evolution_priors.surprise_trigger_threshold();
    // ...
}

// viewport.rs - 修改 refresh_global_attention

pub fn refresh_global_attention(&self, t: u64) {
    // 使用演化先验的空间注意力
    for (i, slot) in self.slots.iter().enumerate() {
        let s = slot.read().unwrap();
        if s.active {
            if let Some(sp) = &s.spatial {
                let bias = self.evolution_priors.spatial_attention((sp.pos_x, sp.pos_y));
                // 应用到注意力计算
            }
        }
    }
}
```

### 2.5 设计原则验证

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 无语义标签 | ✅ | vib_band_sensitivity 是数值数组，不是"情绪" |
| 参数是时间曲线 | ✅ | 先验值会在运行时被 EMA 更新 |
| 无 concept_id | ✅ | 没有引入语义层 |
| 外部描述 | ✅ | 先验是工程参数，不是内部 ontology |

---

## Task 3: Integration & Verification

### 3.1 在 body.rs 中集成

```rust
// bin/body.rs - 添加 tick

// Slot 动态调整 ticker, 1 Hz
let vp_slots = vp.clone();
let h_slot_dynamics = spawn_ticker(1, stop.clone(), move |_| {
    vp_slots.tick_slot_dynamics(now_us());
});
```

### 3.2 Snapshot 兼容

```rust
// snapshot.rs - 读取时兼容新旧格式

fn read_slots(buf: &[u8]) -> Result<Vec<ObjectSlotSnapshot>> {
    // 如果是旧格式（固定4个），转换为动态
    // 如果是新格式，直接读取长度
}
```

### 3.3 Observer 扩展

```rust
// observer.rs - 显示动态 Slot 信息

println!("\tslots(id:mod|kind|...)"));
// 现在显示实际数量，而不是固定4个
```

---

## 验收标准

| 标准 | 验证方式 |
|------|----------|
| Slot 数量可动态变化 | 运行 body，观察 slot_dynamics 日志 |
| 声音学习更快收敛 | 对比有/无先验的 vocal_memory 增长速度 |
| 无语义标签 | 代码审查：无 emotion/feeling/concept_id 字符串 |
| 内生驱动 | Slot 扩张由内部利用率触发，不是外部命令 |
| 编译通过 | cargo build --bin body 无错误 |
| 测试通过 | cargo test 无失败 |

---

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| 动态 Slot 引入性能问题 | 设置上限 N_OBJECT_SLOTS_MAX=32 |
| 先验导致过度拟合 | 先验值作为初始值，会被 EMA 更新 |
| Snapshot 兼容问题 | 保留新旧格式兼容读取 |
| 违反设计原则 | 代码审查 + 设计原则检查表 |

---

## 依赖

- Task 1 依赖：无
- Task 2 依赖：无
- Task 3 依赖：Task 1 + Task 2

---

## 预计工作量

- Task 1: ~2 小时
- Task 2: ~1.5 小时
- Task 3: ~1 小时
- 总计: ~4.5 小时
