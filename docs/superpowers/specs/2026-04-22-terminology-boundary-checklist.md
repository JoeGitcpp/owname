# 术语边界清单（2026-04-22）

> 状态：工作参考文档（随工程演进更新）
> 基于：
> - [AGENTS.md](../../../AGENTS.md)
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md)
> - [tier0-base-elements.md](../../../tier0-base-elements.md)
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md)
> - [2026-04-20-bionic-v0-architecture-audit-cleanup-design.md](./2026-04-20-bionic-v0-architecture-audit-cleanup-design.md)
> - 所有已冻结 spec（2026-04-20 至 2026-04-21）
>
> 本文目的：把散落在各 spec 的术语边界约定归拢成一份可查阅的清单。
> 本文不修改任何代码，仅作文档整合与缺口定位。

---

## 0. 使用说明

本清单分四节：

1. **仅为工程索引名** — 这些词在文档和代码里出现合法，但不能被解读为内部本体单位。
2. **仅限外部描述使用** — 这些词在 spec / 注释 / 观察侧合法，但不能成为身体内部认知单位、资格门或概念分配器。
3. **理论层底层替换对照** — 当 spec 或代码需要表达某个理论概念时，理论层应使用的底层说法。
4. **待补边界注释的位置** — 当前已发现的、缺少边界说明的代码或文档位置。

**可引用但不可替代理论结构**的规则：第 2 节中所有词都是"可引用但不可替代理论结构"的词。它们可以出现在 spec 的"外部观察术语说明"里、出现在观察侧（observer）的渲染逻辑里，但不能成为以下任何一种：
- 函数名 / 字段名（如 `imitation_score`、`curiosity_level`）
- 分配器入口（如"对象先获得 concept_id 才可被指称"）
- 资格门（如"通过语义门才进入认知"）
- 固定世界脚本（如"这是探索目标范围"）

---

## 1. 仅为工程索引名

这些词是工程代码中的**结构句柄、文件名、常量名、字段名或枚举变体**。它们指向基础设施，不代表身体内部真实本体单位。在文档中引用它们时，必须配合"工程索引名"或类似标注，不能被误读为"系统真正在内部用这些词代表什么"。

### 1.1 文件 / 模块名

| 工程索引名 | 所指向的底层能力 | 边界说明 |
|---|---|---|
| `adapter.rs` | 感知 ingress 适配层 | 文件名，不是认知层级标签 |
| `viewport.rs` | 共享视口（对象/关系/时间结构） | 文件名 |
| `snapshot.rs` | 观察侧状态导出机制 | 文件名，不是身体本体 |
| `language_binding.rs` | 外部模式-内部对象绑定轨迹 | 文件名；绑定轨迹本身是学习结构，不是语言本体 |
| `action_output.rs` | 写出历元 substrate | 文件名；"action"是外部观察术语 |
| `action_consequence.rs` | 后果窗口 substrate | 文件名；"consequence"是外部观察术语 |
| `cleanup_hygiene.rs` | 防回退测试支架 | test-only |
| `ordered_global_refresh.rs` | 全局刷新顺序测试支架 | test-only |
| `synthetic_vib.rs` / `phot.rs` | 合成感知源（测试用） | test-only / synthetic |

### 1.2 常量名

| 常量名 | 含义 | 边界说明 |
|---|---|---|
| `HISTORY_CAP` | AxisRing 环形缓冲上界 | 纯工程上界，不代表"历史重要性截止点" |
| `REASONING_HISTORY_RING_CAP` | 推理历史环大小 | 同上 |
| `CONSEQUENCE_WINDOW_CAP` | 后果窗口 Vec 上界 | 工程近似，见 epoch/window spec §3 |
| `PAIR_HISTORY_CAP` | 配对记录 Vec 上界 | 工程近似 |
| `TEMPORAL_PAIR_MAX_DELAY_US` | 时间配对最大延迟 | 工程可调参数，不是"后果窗口的本体边界" |

### 1.3 结构体 / 枚举变体名

| 工程名 | 边界说明 |
|---|---|
| `ExternalPatternKind::SymbolSequence` | 外部模式分类枚举变体。`Symbol` 在此只是分类标签（工程 enum variant），不是"内部符号本体单位"，不得被引申为 token 本体论入口 |
| `ExternalPatternKind::VisualGlyph` | 同上，外部模式分类，不是"身体看到了字形"的本体断言 |
| `ConsequenceModalityKind::GenericIngress` | 工程枚举，用于区分接入类型，无意义语义 |
| `ConsequenceWindowPhase::Open/Closed` | substrate 生命周期阶段，不是"后果评估结果" |
| `ConsequencePairStatus::Pending/Paired` | 配对状态追踪，不是"关系是否成立"的本体判断 |
| `BindingStatus::Candidate/Reinforced/Weakened` | 绑定强度追踪状态，不是"语言理解程度" |
| `GlobalReasoningMemorySummary` | 数值时间底座的摘要结构，`memory` 在此只是工程名，不是语义记忆 |
| `AxisRing` | 环形历史缓冲，纯工程结构 |
| `write_epoch_history` (Vec 字段) | 写出历元账本，工程字段名 |
| `windows` (ActionConsequenceState Vec) | 后果窗口集合，工程字段名 |
| `pairs` (ActionConsequenceState Vec) | 配对记录集合，工程字段名 |
| `window_index` | Vec 位置索引，不是稳定 ID，不是本体引用 |

### 1.4 observer 侧函数名（合法）

以下函数和标签存在于 `bin/observer.rs`（研究观察侧），在此出现**合法**，因为 observer 是外部研究工具，允许生成人类可读输出：

| 函数名 | 说明 |
|---|---|
| `health_label` / `label_health` | 为研究侧渲染健康状态文字 |
| `binding_pattern_label` | 渲染外部模式种类缩写 |
| `binding_status_label` | 渲染绑定状态缩写 |
| `binding_target_label` | 渲染绑定目标摘要 |

**边界**：这些 `label_*` 函数不能反向进入 `body` 进程，也不能被误认为"身体在内部用标签理解自己的绑定状态"。

---

## 2. 仅限外部描述使用

这些词可以出现在以下场合：
- spec 的"高风险术语边界说明"节中（明确说明是外部观察术语）
- 文档中对系统行为的外部描述
- observer / 研究侧注释

**不得**出现在以下场合：
- 正式理论层的 struct / field / function 名（没有边界注释的情况下）
- 身体 substrate 的逻辑路径（作为资格条件、分类器、评估器）
- 对象或关系的"内部真实身份"字段

### 2.1 通用高风险词（来自 AGENTS.md §8.3 + 各 spec §1.x）

| 词 | 允许出现场合 | 禁止出现场合 |
|---|---|---|
| `semantic` / `语义` | 外部描述、拒绝旧架构说明 | 身体内部字段、资格门 |
| `concept` / `概念` | 外部描述 | 内部概念编号器、对象/关系身份 |
| `token` | 外部描述、拒绝旧架构说明 | 认知单位、内部本体 |
| `label` / `标签` | observer 侧渲染、外部描述 | 身体内部分类器 |
| `name` / `名称` | 外部描述 | 对象命名资格门 |
| `word` / `词` | 外部描述 | 语言本体单位 |
| `text understanding` / `文本理解` | 外部描述 | 任何内部层 |
| `identity` / `身份` | 外部描述 | 对象/关系的内部真实身份字段 |
| `owner` / `主体` | 外部描述 | 所有权门 |
| `ethics` / `伦理` | 外部描述 | 内部评估层 |
| `personality` / `人格` | 外部描述 | 内部状态字段 |

### 2.2 行为描述词（来自各 spec §1.3 高风险术语边界说明）

| 词 | 允许出现场合 | 禁止出现场合 |
|---|---|---|
| `模仿` / imitation | spec 外部描述，observer 注释 | 内部模仿器官、模仿评分字段 |
| `后果` / consequence | spec 外部描述，结构名（已加边界注释）| 结果裁决层、意义字段 |
| `比较` / comparison | spec 外部描述 | 内部比较模块 |
| `保持` / retention | spec 外部描述，工程上界名 | 重要性保留策略（Learned Boundaries Only）|
| `回流` / feedback loop | spec 外部描述 | 专用自我感知模块名 |
| `探针` / probe | spec 外部描述 | 探针功能引擎 |
| `好奇心` / curiosity | spec 外部描述 | 好奇心字段、好奇心参数 |
| `探索` / exploration | spec 外部描述 | 探索范围固定字段 |
| `自稳` / self-stabilization | spec 外部描述 | 独立自稳模块 |
| `自我调整` / self-adjustment | spec 外部描述 | 独立调整器 |
| `局部修复` / local repair | spec 外部描述 | 独立修复器官 |
| `主动性` / initiative / agency | spec 外部描述 | 主动性字段、主动性计分器 |
| `记忆` / memory（语义义） | observer 侧，spec 外部描述 | 语义记忆单位（数值底座中的 `memory` 可用，但须注明是时间底座） |

---

## 3. 理论层底层替换对照

当 spec 或代码需要表达某个理论概念时，理论层不应直接复述外部描述词，而应使用以下底层说法。

| 外部描述词（不用于理论层） | 理论层应使用的底层说法 |
|---|---|
| 模仿 | `predictor / err / precision / update` + 曲线逼近（curve approximation on existing substrate） |
| 后果（语义层含义） | 时间窗 / 延迟结构 / 曲线片段 + viewport 状态变化 |
| 比较（自身 vs 外部） | predictor err 差值 / update 信号差值 |
| 好奇心 | 未闭合预测 / 局部高不确定性 / 长期未确认关系结构 / 历史上可降低不确定性的区域 |
| 探索时机 | 结构转折条件（structural inflection condition），不是数字门 |
| 重要性 | ❌ 不得预设。属于 Learned Boundaries Only，不能以固定字段或常量出现在理论层 |
| 语义理解 | 绑定强度（`BindingStrength.value`）/ 绑定历史 / 绑定修正轨迹 |
| 概念 | 对象 / 关系 / 时间结构（object / relation / time-structure） |
| 词义 | 对象-关系网络的压缩访问（compressed access to object-relation network） |
| 意图 | ❌ 不得预设。属于 Learned Boundaries Only |
| 目标 | ❌ 不得预设。属于 Learned Boundaries Only |
| 主动性 | 内生活动强度（endogenous activity level）+ 结构转折条件 |
| 记忆（语义义） | `GlobalReasoningHistoryState`（数值时间底座）/ 推理历史环缓冲 |
| 保留策略（优先级义） | 工程近似 cap / ring / 活跃扫描集分离——不得引申为"更重要的历史" |
| 快捷路径（神经通路义） | 高频绑定加固路径（high-frequency binding reinforcement path）——始终只是访问捷径，不能反向成为内部概念本体 |
| 自我感知 | 自身输出物理后果通过相同感知 ingress 进入身体（可学习区分，不是先验本体） |

---

## 4. 待补边界注释的位置

以下位置当前缺少或不完整的边界注释，建议补充：

### 4.1 `bionic_v0/src/language_binding.rs`

**缺口**：`ExternalPatternKind::SymbolSequence` 枚举变体没有注释说明它是外部模式分类（工程 enum），不是内部符号本体单位。

**建议在 `ExternalPatternKind` 枚举定义处加注释：**
```rust
// Engineering index: classifies the kind of external pattern presented
// during binding. These variants describe the *source channel*, not internal
// ontology units. `SymbolSequence` is a channel classifier, not a token-
// ontology entry point.
```

### 4.2 `bionic_v0/src/viewport.rs` — `GlobalReasoningMemorySummary`

**缺口**：`GlobalReasoningMemorySummary` 和 `GlobalReasoningHistoryState` 中出现的 `memory` 字样，代码注释写了"numeric-only temporal memory substrate"，但结构体文档注释未明确说明 `memory` 此处仅是工程字段名，不是语义记忆。

**建议在 `GlobalReasoningMemorySummary` struct 注释处补充：**
```rust
// "memory" here is an engineering label for this numeric-time-only
// substrate slot. It is not a semantic-memory unit and does not imply
// that the body internally stores "what things mean".
```

### 4.3 `bionic_v0/src/viewport.rs` — `AxisRing`

**现状**：已有 `HISTORY_CAP` 注释，但未明确说明 `HISTORY_CAP` 是工程上界而非"历史重要性截止点"。

**建议在 `HISTORY_CAP` 常量定义处（`lib.rs`）加注释：**
```rust
// Engineering cap only. Does not imply that samples beyond this window
// are less important — importance is not pre-assigned by this substrate.
```

### 4.4 `AGENTS.md` — §8.3 高风险词列表

**缺口**：§8.3 只列出了需要四点解释的高风险词，但没有"正确底层替换说法"的对照。读者知道不能用这些词做什么，但不知道应该改成什么。

**建议在 §8.3 末尾增加一行指向本文档的引用：**
```markdown
正确底层替换对照见：
[docs/superpowers/specs/2026-04-22-terminology-boundary-checklist.md §3](docs/superpowers/specs/2026-04-22-terminology-boundary-checklist.md)
```

### 4.5 各 spec 的"高风险术语边界说明"节

各 spec（`action-output-substrate`、`action-consequence-loop`、`endogenous-activity-and-probe`、`epoch-and-window-history-retention`）已经有各自的 §1.x 边界说明节，**不需要修改**——它们说明了各自 spec 范围内的术语边界。

本文档的作用是补充一个**全工程统一视图**，供跨 spec 查阅。

---

## 5. 已正确实现的边界模式（参考）

以下是当前代码库中**已正确处理术语边界**的示例，供后续新代码参考：

| 位置 | 正确做法 |
|---|---|
| `adapter.rs:12`, `viewport.rs:264`, `object_high_layer.rs:12` | 文件头注释显式写明 `No semantic labels, no tracking IDs — numeric only` |
| `synthetic_vib.rs:11` | `Low-level numeric output only. No labels, no object names.` |
| `action_output.rs:58,86` | struct 字段注释写明 `no target, concept, or meaning semantics` |
| `action_consequence.rs:35` | 枚举注释写明 `No result, success, target, or meaning semantics` |
| `viewport.rs:634–637` | 显式注释解释了为何删除 SemanticReadinessState 和 SymbolizerState |
| `lib.rs:287–290` | 注释说明 Phase 5H/6A 被拒绝的原因 |
| `bin/observer.rs` | label 函数全部在 observer 侧，body 进程不含任何 `label_*` 调用 |
