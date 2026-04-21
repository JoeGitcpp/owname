# 仿生意识流 v0 · 设计 Spec

> **创建**：2026-04-19
> **状态**：⚠️ **已被 2026-04-20 清理 spec 覆盖部分内容（DEPRECATED IN PART）**
> **基于**：
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md) v1.0
> - [tier0-base-elements.md](../../../tier0-base-elements.md) v0.8
> **必读后继文档（优先级高于本文件）**：
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md) — 语言系统公理
> - [2026-04-20-bionic-v0-architecture-audit-cleanup-design.md](./2026-04-20-bionic-v0-architecture-audit-cleanup-design.md) — 架构清理 spec

---

## ⚠️ 0. 弃用说明（READ FIRST）

本文件大部分内容仍可参考，但以下条款**已被清理 spec 拒绝**，不再代表当前架构方向。任何新工作都必须先读上方两份后继文档，再来读本 spec。

### 0.1 已被拒绝的条款

下列名称在本文件后续段落中仍然出现，但**仅作为历史记录**，不构成 v0 实际器官，也不得被未来 spec 复活：

- **`semantic_readiness`** — 已删除：把命名/标签做成对象之上的资格门，违反语言公理 1、3、5、8。
- **`symbolizer` / `concept_id`** — 已删除：把工程索引冒充成内部"概念"本体，违反公理 3、5、边界 5。
- **`大脑警察` / `ethics_predictor`** — 已降级为高误导命名层；不构成正式器官，未来如有伦理相关需求，需重新立项并先通过最高原则审计。
- **`主人身份` / `主人验证`** — 同上：v0 不含，且未来若引入需重新立项；不得被默认理解为本体内部模块。
- **"屏幕可视化输出"画在 AI 本体框内** — 违反研发观察与本体分离原则。可视化属外部 observer，不属于身体器官。

### 0.2 与现实的漂移

- 本 spec 写"v0 不含跨模态对象绑定"，但当前实现已存在 same-slot 跨模态融合与重用路径；以**实现**为准。
- 本 spec 第 5 节列出了 `refresh_semantic_readiness` 与 `refresh_symbolizer` 进入 body 主循环的设计，**已被移除**；body 主循环终止于 reasoning history + snapshot。

### 0.3 仍然有效的部分

- 视口/感知/slot/融合/关系/推理/历史摘要/共享底座等本体子系统，整体方向有效，但仍需以最新代码与清理 spec 为准。

---

## 1. Spec 目的与范围

### 1.1 目的

本 spec 定义"**仿生意识流 v0**"的**最小可行实现**。它不是完整 AI，是一个证明架构原理可跑通的概念验证（PoC）。

### 1.2 范围

**在 v0 范围内**：
- 视口（公理 8）的最小实现
- 单 modality 起步（VIB 麦克风），逐步加 PHOT、MECH
- VPC 学习算法的核心循环
- 多分辨率两层（低层 + 中层）
- 屏幕可视化输出
- 扬声器输出（加性合成）
- 大脑警察 + 精度战争防护的最小集
- 性格策略接口（默认实现）

**v0 不含**：
- 嗅觉（CHEM）
- 机械臂
- 主人身份验证（v0 无主人，纯实验）
- 完整伦理监测（仅设接口和占位符）
- 多分辨率高层与顶层
- 自我模型
- 跨模态对象绑定
- 性格演化机制
- 物理自毁机制

---

## 2. 系统架构图

```
                     ┌─────────────────────────────────┐
                     │     外部仪表盘（独立进程）         │
                     │   不属于 AI 本体；研发观察工具      │
                     └─────────────▲───────────────────┘
                                   │ 只读快照
                                   │
┌──────────────────────────────────┴────────────────────────────────┐
│                         AI 本体进程                                  │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │                  共享态视口（Viewport）                        │  │
│  │  Tier 0 轴 × 对象 slot × (μ, precision, err, predicted)      │  │
│  └──▲────▲────▲────▲────▲────▲────▲────▲─────────────────────┘  │
│     │    │    │    │    │    │    │    │                          │
│  写读 写读 写读 写读 写读 写读 写读 写读                              │
│     │    │    │    │    │    │    │    │                          │
│  ┌──┴──┐┌┴───┐┌┴───┐┌┴───┐┌┴───┐┌┴───┐┌┴───┐┌──┴──────┐         │
│  │感官  ││低层││中层││动作││大脑││精度││性格││可视化输出│         │
│  │读出器││VPC ││VPC ││输出││警察││战争││策略││（屏幕）  │         │
│  │(VIB) ││    ││    ││(扬声器)│护栏││接口││         │         │
│  └──▲──┘└────┘└────┘└──┬─┘└────┘└────┘└────┘└────────┘         │
│     │                   │                                          │
└─────│───────────────────│──────────────────────────────────────────┘
      │                   │
   麦克风              扬声器
   摄像头
   触摸板
```

---

## 3. 实施阶段（5 阶段）

每阶段独立验证，问题可隔离。

### 阶段 0 · 视口框架（1 周）

**目标**：跑起来一个空视口 + 时钟系统。

**交付物**：
- `Viewport` 数据结构（共享内存，lock-free atomic）
- 多节律时钟系统（1 kHz 主 + 分频）
- 测试 predictor（dummy）证明 read/write 工作
- 没有学习，没有感官，没有输出

**通过标准**：
- 多个 dummy predictor 在不同节律下并发读写视口无 race condition
- 主时钟稳定 1 kHz，jitter < 100 μs
- 视口快照可被外部进程读取

### 阶段 1 · 单 predictor + VIB 模态（2 周）

**目标**：跑通最简单的 VPC 循环——一条 vib_band 轴的预测/学习。

**交付物**：
- 麦克风读出器（CoreAudio → mel 频谱 → vib_band_*）
- 一个 VPC predictor（线性 AR-2 外推器）
- 该 predictor 学习预测下一帧 vib_band 值
- 屏幕显示该 vib_band 的历史曲线 + 预测曲线 + err 热图

**通过标准**：
- 对着麦克风哼一个稳定音 → err 在几秒内收敛到接近零
- 停止哼 → 预测曲线惯性延展几百 ms 后衰减
- 换一个音 → err 立刻爆发，重新收敛

### 阶段 2 · 多 predictor 同模态（2 周）

**目标**：验证机制 5（视口隐式信用分配）真的减少 err。

**交付物**：
- 多个 vib_band predictor（每个频段一个）
- 一个高层 predictor 监听底层 err 模式
- 高层 predictor 学习预测底层 err
- 高层成功预测 err 时，下传 prior 给底层调整

**通过标准**：
- 高层 predictor 加入后，底层平均 err 比单独运行时降低 ≥ 20%
- 移除高层 predictor，底层 err 上升回原水平
- 这证明视口隐式信用分配机制工作

### 阶段 3 · 跨模态（2 周）

**目标**：加入 PHOT 和 MECH 模态，验证视口跨模态融合。

**交付物**：
- 摄像头读出器（OpenCV → 光流 + 颜色直方图 → light_band_* + pos_x/y）
- 触摸板读出器（OpenMultitouchSupport → pos_x/y + span + compress_res）
- 多对象 slot 管理（最多 4 个并行）
- AMBIENT 通道（视觉全幅光流 + 听觉宽频能量包络）

**通过标准**：
- 触摸板触点出现 → 对象 slot 自动分配
- 触点消失 → 对象 slot 自动释放
- AMBIENT 检测到运动 → 提名新 ROI 升级为 MID 档
- 三个模态并发读写视口无冲突

### 阶段 4 · 输出闭环（1 周）

**目标**：从视口到扬声器的反向预测合成。

**交付物**：
- 扬声器输出器（vib_band → 加性合成 → PCM）
- AI 尝试"哼"出它当前预测的下一段声音
- 屏幕渲染所有活对象的 tier 0 状态可视化

**通过标准**：
- AI 听到稳定音 → 扬声器开始发出近似的声音
- 主人哼调子，AI 跟着哼（学习的最低形态："模仿"）
- 屏幕上能看到对象 slot、tier 0 曲线、err 热图、precision 场

### 阶段 5 · 防护与策略接口（1 周）

**目标**：装上精度战争防护和性格策略接口。

**交付物**：
- 精度战争防护：规则 1（感官下限）、规则 2（顶层上限）、规则 5（周期衰减）、规则 6（大脑警察豁免）
- 大脑警察 ethics_predictor 占位符（不做实际伦理判断，但走通监测通道）
- viewport_policy 接口（默认：侦察兵新生儿）
- 切换 policy 实例验证学习行为变化

**通过标准**：
- 强行把某 prior precision 调到 200 → 被 clamp 到 100
- 强行把感官 precision 调到 0 → 被 clamp 到 0.05
- 切换 policy → 学习率/precision 默认值改变，行为可观察改变
- ethics_predictor 占位符的 precision 不被任何 policy 修改

---

## 4. 核心数据结构

### 4.1 Viewport

```
Viewport {
  global_clock: AtomicU64                          // 主时钟（μs since epoch）
  
  objects: Vec<ObjectSlot>                         // 最多 N=4 个活对象
  
  ambient: AmbientChannel {
    motion_field: Array2D<f32>                    // 视觉全幅光流
    audio_envelope: f32                           // 音频宽频能量
  }
  
  ethics_state: EthicsState {
    alarm_triggered: AtomicBool
    suppressed_err_budget: HashMap<AxisId, f32>  // 规则 3 用
  }
}

ObjectSlot {
  id: ObjectId
  is_actual: bool                                 // 真实 vs 假设（v0 总是 true）
  axes: HashMap<AxisId, AxisState>
  precision_per_axis: HashMap<AxisId, f32>
  metadata: ObjectMetadata
}

AxisState {
  curve_history: RingBuffer<(t: u64, value: f32, observed: bool)>  // 滑窗
  predicted_curve: Vec<(t: u64, value: f32)>                       // 未来预测
  current_err: f32
  evidence_ptr: Option<StreamPtr>                                  // 可追溯原 Stream 切片
}
```

### 4.2 Predictor

```
trait Predictor {
  fn subscribes_to(&self) -> Vec<AxisId>;
  fn publishes_to(&self) -> Vec<AxisId>;
  fn tick_rate_hz(&self) -> f32;
  fn resolution_threshold(&self) -> f32;
  
  fn tick(&mut self, viewport: &Viewport, policy: &dyn ViewportPolicy) {
    let inputs = viewport.read(self.subscribes_to());
    let pred = self.predict(inputs);
    let observed = viewport.read_at_time(self.publishes_to(), now + delta);
    let err = observed - pred;
    if err.abs() > self.resolution_threshold() {
      let lr = policy.get_learning_rate(self.layer(), self.axis(), context);
      let precision = policy.get_precision(self.layer(), self.axis());
      self.update_local(err, lr, precision);
    }
    viewport.write(self.publishes_to(), pred, precision);
  }
  
  fn predict(&self, inputs: Inputs) -> Output;
  fn update_local(&mut self, err: f32, lr: f32, precision: f32);
}
```

### 4.3 ViewportPolicy（性格接口）

```
trait ViewportPolicy {
  fn get_learning_rate(&self, layer: LayerId, axis: AxisId, context: Context) -> f32;
  fn get_precision_init(&self, layer: LayerId, axis: AxisId) -> f32;
  fn get_aggressiveness(&self, context: Context) -> f32;
  fn get_attention_weights(&self) -> AttentionWeights;
}

struct DefaultScoutPolicy;  // 侦察兵新生儿
impl ViewportPolicy for DefaultScoutPolicy { ... }
```

---

## 5. 实现技术栈

### 5.1 推荐栈

| 组件 | 技术 | 理由 |
|---|---|---|
| 主语言 | **Rust** 或 **Swift** | 性能 + 安全 + 多线程 |
| 视口共享内存 | mmap + atomic | 跨进程 + 高性能 |
| 音频 | CoreAudio（macOS） | 低延迟 |
| 视频 | AVFoundation（macOS） + OpenCV | 摄像头 + 光流 |
| 触摸板 | OpenMultitouchSupport（Swift 包）| 直接读取 |
| 屏幕渲染 | Metal / WebGL | 用户已有栈 |
| 外部仪表盘 | 独立 Web 进程 | 不污染 AI 本体 |

### 5.2 备选栈

| 组件 | 备选 | 何时考虑 |
|---|---|---|
| 主语言 | Python + numba | 快速原型 |
| 视口 | Redis 共享数据结构 | 跨机器 |
| 屏幕渲染 | Filament | 用户上一 session 在用 |

---

## 6. 验收标准（v0 整体）

### 6.1 行为级验收

| # | 场景 | 期望行为 |
|---|---|---|
| 1 | 哼一个稳定音 | err 收敛 + 扬声器跟着哼 |
| 2 | 突然换音 | err 爆发 + 扬声器跟随调整 |
| 3 | 触摸板触点 | 对象 slot 分配，pos 曲线在视口出现 |
| 4 | 摄像头前挥手 | AMBIENT 通道触发，新对象 slot 提名 |
| 5 | 强制 precision 越界 | 被 clamp，AI 不进入幻觉 |
| 6 | 切换 policy | 学习率改变，行为可观察改变 |

### 6.2 数值级验收

| # | 指标 | 阈值 |
|---|---|---|
| 1 | 主时钟 jitter | < 100 μs |
| 2 | 单 predictor 每 tick 时间 | < 0.1 ms |
| 3 | 完整循环延迟（感官→输出）| < 50 ms |
| 4 | 阶段 1 单音 err 收敛时间 | < 10 秒 |
| 5 | 阶段 2 高层加入后 err 降低 | ≥ 20% |
| 6 | 阶段 3 多模态并发吞吐 | 全部模态按其 tick rate 不丢 |
| 7 | 内存占用 | < 2 GB |
| 8 | CPU 占用 | < 60%（MacBook M 系列）|

### 6.3 哲学级验收

| # | 检查项 | 标准 |
|---|---|---|
| 1 | AI 本体无人类语言 | 源代码 grep 无 print/log/字符串字面量 |
| 2 | 视口为唯一通信 | 无 predictor 之间直接调用 |
| 3 | 永远在跑 | 所有 predictor 持续运行，无 batch/epoch 概念 |
| 4 | 全本地 | 无跨 predictor 的链式法则梯度 |
| 5 | 性格可热替换 | 切换 policy 实例无需重启 AI |

---

## 7. 风险与缓解

| 风险 | 严重度 | 缓解 |
|---|---|---|
| VPC 没有先例验证 | 🔴 极高 | 阶段化验证；阶段 1 不通过则修正基础 |
| 视口竞态条件 | 🟡 高 | 严格使用 atomic / lock-free 数据结构 + 阶段 0 充分压测 |
| 多节律时钟漂移 | 🟡 中 | 全局 wall-clock 校准 + jitter 监控 |
| 阶段 2 隐式信用分配不工作 | 🔴 高 | 该机制是 VPC 核心，不工作则需重新设计；准备 PCN 退路 |
| 跨模态绑定不稳 | 🟡 中 | 多对象 slot 加 hysteresis 阈值 |
| 屏幕可视化复杂度爆炸 | 🟢 低 | v0 只显示数值热图，不做艺术化 |
| 用户主人身份缺失 | 🟢 低 | v0 没有主人，AI 进入"无主人"待机模式即可 |

---

## 8. 开放问题（不阻塞 v0，但需后续解决）

1. 主人身份验证的硬件实现细节（v1+）
2. 历史灾难知识库的内容和编码方式（v1+）
3. 跨模态对象绑定的详细算法（v0 占位符即可）
4. 性格系统的初始性格谱系（v1+）
5. 自我模型的具体表征（v2+）

---

## 9. 与已锁文档的引用关系

- **Tier 0 元素表** → [tier0-base-elements.md](../../../tier0-base-elements.md) v0.8
  - 本 spec 引用 tier 0 的 32 固定轴 + 8 频带轴组
  - 本 spec 中的 AxisId 等价于 tier 0 中的 8-bit 编码

- **架构原则** → [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md) v1.0
  - 本 spec 是该架构原则的 v0 实现契约
  - 任何架构争议以原则文档为准
  - 本 spec 不重复解释原则，仅引用

---

## 10. v0 之后的演化路径（仅参考）

```
v0 (本 spec)
   ↓ 验证 VPC 核心机制
v1: 加入对象绑定 / 自我模型 / 主人身份验证 / 完整伦理监测
   ↓
v2: 加入机械臂 / 因果学习 / 高层抽象推理
   ↓
v3: 加入梦境模式 / 性格演化 / 多 AI 协作
   ↓
v4+: 探索硅基生命形态
```

每个版本都是一个独立的 spec，等本 v0 验证后再启动下一个。

---

## 11. 文档版本

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-04-19 | 初版冻结，可进入 writing-plans 阶段 |
