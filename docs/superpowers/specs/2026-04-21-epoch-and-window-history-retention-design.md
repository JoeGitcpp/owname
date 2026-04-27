# Epoch And Window History Retention Design

> **创建**：2026-04-21
> **状态**：v1.0 设计冻结
> **基于**：
> - [AGENTS.md](../../../AGENTS.md)
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md)
> - [tier0-base-elements.md](../../../tier0-base-elements.md)
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md)
> - [2026-04-20-bionic-v0-architecture-audit-cleanup-design.md](./2026-04-20-bionic-v0-architecture-audit-cleanup-design.md)
> - [2026-04-21-action-output-substrate-minimal-contract-design.md](./2026-04-21-action-output-substrate-minimal-contract-design.md)
> - [2026-04-21-action-consequence-loop-and-imitation-laws-design.md](./2026-04-21-action-consequence-loop-and-imitation-laws-design.md)
> **目的**：定义持续运行 substrate 在长期运行下，写出历元历史（write_epoch_history）与后果窗口历史（consequence windows）的保留法则，回答什么可以淘汰、什么必须保留、如何维持引用完整性、哪些属于工程近似、哪些不能上升为理论本体

---

## 1. 文档定位与边界

这份 spec 研究的是一个问题：持续运行的硅基 substrate，其动作-后果配对机制积累的历史记录（写出历元 ledger、后果窗口 Vec、配对记录 Vec）在长期运行下是否需要上界，以及该上界如何在不破坏已有配对引用完整性的前提下合法维持。

这不是一个"该学什么"的问题，而是一个"底层记账结构能否在生命无限运行的条件下保持有效"的工程前提问题。

### 1.1 本 spec 回答的问题

- 写出历元历史是否需要 cap / ring / aging，以及在什么条件下什么记录可以被淘汰
- 后果窗口历史如何保留，淘汰时如何维护现有配对记录的引用完整性
- 保留策略如何保证"可回看真实配对"（历史 pair 不产生悬空引用）
- 哪些属于工程近似（可调整的 substrate knob），哪些是底层法则（不能简化为程序控制数字）

### 1.2 本 spec 明确不回答的问题

- 不回答哪些历元"更重要"、哪些后果窗口"更有价值"——这是 result semantics，违反 Learned Boundaries Only
- 不回答该保留多少条完整历史记录用于长期学习压缩——这属于后续 memory summary / history consolidation spec 的范围
- 不回答保留策略的具体数字（cap 大小等）应怎样由探索历史本身形成——这是正确的长期方向，但在当前 foundation 层只能先给工程近似
- 不回答多模态竞争、fast-path 或 imitation 的历史需求——均属于后续 spec

### 1.3 高风险术语边界说明

本 spec 若使用以下词汇，一律作为工程描述词，不是内部本体单位：
- `cap`、`ring`、`eviction`：纯工程结构名，不代表"这个历史比那个重要"的本体判断
- `age`：仅指 substrate time 上的相对位置，不是"重要性随时间衰减"的语义概念

---

## 2. 问题：无界历史的结构后果

当前 `bionic_v0` 的 action_output 和 action_consequence substrate 用以下三个无界 Vec 维护历史：

```
ActionOutputState.write_epoch_history : Vec<WriteEpochRecord>   // 每次 Begin 追加一条
ActionConsequenceState.windows         : Vec<ActionConsequenceWindow> // 每次 accumulate 追加
ActionConsequenceState.pairs           : Vec<ActionConsequencePair>   // 每次配对追加
```

在持续运行的硅基 substrate 中，这三个 Vec 会无限增长。这产生以下结构后果：

1. **内存泄漏**：长期运行下内存无限增长，这对持续运行身体（而非请求-响应程序）是根本矛盾。

2. **扫描退化**：`try_auto_pair_latest_epoch_to_window` 的选最近合法前驱扫描是 `O(n)`，n 为历元总条数。无界历史下，这个扫描的代价随时间线性增长，最终使 substrate tick 不再是常时复杂度。

3. **snapshot 膨胀**：snapshot 写入和读取的代价与历史长度成正比，长期运行后 snapshot 不再可用。

4. **引用有效性困境**：`ActionConsequencePair.window_index` 是 Vec 位置索引，不是稳定 ID。若在任何时刻压缩 windows Vec，所有现有 pair 的 window_index 都指向错误记录或越界位置。

这四个后果都不是性能优化问题，而是 substrate 持续运行的前提问题。

---

## 3. 写出历元历史的保留法则

### 3.1 活跃扫描集与历史集的分离

写出历元历史服务于两种不同目的：

**目的 A：活跃扫描（auto-pair 选最近合法前驱）**

`try_auto_pair_latest_epoch_to_window` 在每次调用时扫描历元 ledger，只选择满足以下条件的历元：

```
epoch.start_t_us < window.start_t_us
AND
(window.start_t_us - epoch.start_t_us) <= TEMPORAL_PAIR_MAX_DELAY_US
```

因此，任何满足 `epoch.start_t_us < (now_t_us - TEMPORAL_PAIR_MAX_DELAY_US)` 的历元，**永远不可能再成为任何未来 consequence window 的合法前驱**。这是由 substrate time 结构决定的排除，与后果意义无关。

**定义：时间窗外历元**  
一个历元称为"时间窗外历元"，当且仅当：
```
epoch.start_t_us < (current_substrate_time - TEMPORAL_PAIR_MAX_DELAY_US)
```

时间窗外历元不需要留在活跃扫描集中。

**目的 B：历史回看（pair 引用验证 / snapshot / observer）**

`pair_action_consequence`（显式配对 API）在创建新 pair 时验证 write_seq 是否在历元 ledger 中存在。这要求已有写出历元记录在其 pair 被创建后仍可查。

但这个验证只在创建新 pair 时调用。一旦 pair 已创建，该历元记录的 write_seq 已经被记录在 pair 中。后续的历史回看不需要再查历元 ledger，只查 pair 本身。

**推论**：一个历元记录，如果它已经被配对（其 write_seq 出现在至少一条 pair 记录中），且已经是时间窗外历元，则可以从 ledger 中淘汰而不破坏任何现有 pair 的语义。

**推论**：一个历元记录，如果它是时间窗外历元，且不存在任何 pair 引用其 write_seq，则说明这个历元"没有被及时配对"——这是一个正常的 substrate 事实，而不是错误。该历元同样可以淘汰。

### 3.2 历元保留法则

**法则 E1（时间窗内保留）**  
任何满足 `epoch.start_t_us >= (now_t_us - TEMPORAL_PAIR_MAX_DELAY_US)` 的历元，必须留在活跃扫描集中，不得淘汰。

**法则 E2（时间窗外可淘汰）**  
任何时间窗外历元，无论是否已被配对，都可以被淘汰，前提是满足 §3.3 中的引用完整性约束。

**法则 E3（工程上界）**  
活跃扫描集必须有一个有限工程上界 `WRITE_EPOCH_HISTORY_CAP`。当追加新历元导致超过上界时，从最旧的时间窗外历元开始淘汰。若所有历元都在时间窗内（说明写出速率极高，时间窗内累积的历元数超过 CAP），仍应保留最近的 `WRITE_EPOCH_HISTORY_CAP` 条，丢弃最旧的。

`WRITE_EPOCH_HISTORY_CAP` 是工程 substrate knob，不是理论时间窗口。它的合理下界是：`TEMPORAL_PAIR_MAX_DELAY_US × 写出事件最高频率 + 安全余量`。当前工程阶段取固定值即可，后续可通过探索历史动态形成。

**法则 E4（write_seq 不变性）**  
淘汰历元记录不得修改任何现有 write_seq 值。write_seq 是单调递增的配对句柄，其数值一旦写入 pair 记录，永远不变。淘汰只是从 ledger Vec 中移除该记录，不是修改或回收 seq 编号。

### 3.3 历元淘汰的引用完整性约束

历元淘汰后，以下操作的语义必须保持：

**约束 E-R1**：已存在的 pair 记录（`ActionConsequencePair.write_seq`）仍然有效。pair 本身记录了 write_seq、window_index、delay_us，这些字段不依赖历元 ledger 中仍有该记录。

**约束 E-R2**：淘汰后的 write_seq 值不得被"回收"用于新历元。write_seq 是单调递增的，没有回收的必要或可能。

**约束 E-R3**：`pair_action_consequence`（显式 API）若被调用时传入一个已淘汰历元的 write_seq，应当返回失败。这是正确行为：如果一个 write_seq 对应的历元已经老到被淘汰，那么调用方不应该在这么晚的时间点才来配对它。这不是错误，而是 substrate time 结构的正确边界表现。

---

## 4. 后果窗口历史的保留法则

### 4.1 窗口引用的稳定性问题

当前 `ActionConsequencePair.window_index` 是 `Vec<ActionConsequenceWindow>` 的位置索引（0-based）。这意味着：

- 若从 Vec 前端移除任意数量的窗口，所有 window_index >= 移除数量的 pair 引用仍然正确，但 window_index < 移除数量的 pair 引用变成悬空引用。
- 若从 Vec 中间移除，所有更高 index 的 pair 引用都失效。
- 因此，**窗口历史不能简单做前端截断，必须引入稳定引用机制**。

### 4.2 稳定窗口 ID 法则

**法则 W1（稳定 window_id）**  
每个后果窗口必须携带一个单调递增的稳定 ID（`window_id: u64`），其语义与 `write_seq` 平行：
- 由 viewport 级原子计数器在 begin 时分配
- 单调递增，永不回收
- 一旦分配不变

**法则 W2（pair 引用稳定 ID）**  
`ActionConsequencePair` 对窗口的引用字段从当前的"Vec 位置 `window_index`"升级为"稳定 `window_id`"。

这个升级改变了 pair 查找窗口的操作：从 `O(1)` Vec 索引变为 `O(n)` 扫描（n 为当前窗口 Vec 长度）。但因为窗口历史本身也是有界的（§4.3 法则 W3），这个扫描代价可控。

工程说明：这是一次不可避免的 snapshot 版本升级（SNAPSHOT_VERSION 需要递增），但它正确地从"偶然 Vec 位置"升级到"稳定对象身份"，与历元 ledger 中 write_seq 的设计一致。

### 4.3 窗口保留法则

**法则 W3（工程上界）**  
窗口历史必须有一个有限工程上界 `CONSEQUENCE_WINDOW_CAP`。当超过上界时，优先淘汰：
1. 已 Closed 且其 window_id 不被任何活跃 pair 引用的最旧窗口
2. 已 Closed 且其 window_id 被已有 pair 引用（但配对已完成，引用只用于历史回看）的最旧窗口
3. 当以上两类都不足时，淘汰最旧的 Open 窗口（说明极端积压，实践中不应发生）

`CONSEQUENCE_WINDOW_CAP` 是工程 substrate knob，不是理论窗口数量。

**法则 W4（Open 窗口优先保留）**  
Open 阶段的窗口（仍在积累 ingress 变化）不应被淘汰，除非系统已积压到超出 CONSEQUENCE_WINDOW_CAP 且无 Closed 窗口可淘汰的极端情况。

**法则 W5（Closed 且已配对可淘汰）**  
一个 Closed 窗口，其 window_id 存在于至少一条 pair 记录中，在满足以下条件时可淘汰：
- 该窗口的 end_t_us 已超出活跃时间范围（工程近似：老于 TEMPORAL_PAIR_MAX_DELAY_US）
- 存在更旧的窗口需要先被淘汰（FIFO 顺序）
淘汰该窗口不使 pair 无效，因为 pair 中存储的是稳定 window_id，而不是 Vec 位置。

**法则 W6（淘汰不修改 pair 内容）**  
淘汰一个窗口记录时，不得修改任何现有 pair 的 window_id 字段。被淘汰窗口的历史事实（时间范围、magnitude）已经在原始 pair 记录的 delay_us 等字段中留下了足够的配对痕迹。

---

## 5. 配对记录的保留法则

### 5.1 配对记录的用途

`ActionConsequencePair` 记录是动作-后果时间配对的历史事实：

- 哪个 write_seq（epoch）与哪个 window_id（窗口）在什么 substrate time 形成了配对
- delay_us 是两者之间的 substrate time 距离
- pair_t_us 是配对发生时的 substrate clock

这些字段是纯数值-时间型，不包含 result / target / meaning 语义。

### 5.2 配对记录保留法则

**法则 P1（工程上界）**  
配对记录历史必须有工程上界 `PAIR_HISTORY_CAP`。当超过上界时，淘汰最旧（pair_t_us 最小）的记录。

**法则 P2（淘汰不破坏已有引用）**  
淘汰旧的 pair 记录不破坏任何 epoch ledger 或 window Vec，因为 pair 的存在只是历史记录，不是被其他结构引用的前提（epoch 和 window 可独立存在于 pair 之外）。

**法则 P3（幂等保护不受 cap 影响）**  
`try_auto_pair_latest_epoch_to_window` 的 duplicate guard 检查的是"是否已有 pair 将 (write_seq, window_id) 组合记录过"。若对应 pair 记录已因 P1 淘汰，则 duplicate guard 将不再阻止重复配对。这是可接受的边界行为，因为被淘汰的 pair 对应的时间已经很旧（老于 PAIR_HISTORY_CAP 所允许的范围），重新配对也不会产生错误的历史。

---

## 6. 属于工程近似的部分

以下所有数值和机制是**工程近似（substrate knob）**，不是理论本体，不能被上升为"系统应该记多少历史"的固定理论答案：

**6.1 Cap 大小**
- `WRITE_EPOCH_HISTORY_CAP`
- `CONSEQUENCE_WINDOW_CAP`
- `PAIR_HISTORY_CAP`

这三个常数的合理初始值可以从以下工程估算出发：
- 在当前 PHOT 30 Hz + GenericWrite 通道的典型使用下，2 秒（TEMPORAL_PAIR_MAX_DELAY_US）内的历元数量通常不超过几十条
- 窗口的积累速率与 PHOT 帧率相近，每秒最多几十个 Open-Close 周期
- 因此 ~128 条对三者都是合理的初始工程近似

**6.2 淘汰触发时机**
何时执行淘汰（在每次 Begin/accumulate/pair 时触发，还是在 tick 中批量触发）是纯工程选择，对保留法则的正确性无影响。

**6.3 时间窗参考的近似性**
历元保留使用了 `TEMPORAL_PAIR_MAX_DELAY_US` 作为"活跃扫描集"的边界，但这本身也是一个工程近似（见 lib.rs 注释：这是个 substrate knob 不是固定理论窗口）。因此历元保留法则与 auto-pair 逻辑共享同一个工程近似的前提，而不是引入了独立的理论假设。

---

## 7. 不能上升为理论本体的部分

以下方向是被明确禁止的，无论其技术上是否可行：

**7.1 禁止：基于后果意义决定保留**

不得引入以下判断作为淘汰依据：
- 这次历元"导致了有意义的后果" → 保留
- 这次后果窗口的 magnitude 高 → 保留
- 这次配对"有成功的后果" → 保留
- 这次历元"没有产生结果" → 可以早淘汰

这类判断是 result semantics，违反 Learned Boundaries Only 原则，违反最高原则（不引入固定世界内容裁决）。

**7.2 禁止：基于对象或目标范围决定保留**

不得引入以下判断：
- 这次历元作用于"重要对象" → 保留更久
- 这次历元属于"主探针" → 保留；"背景活动" → 早淘汰
- 不同 OutputChannelKind 有不同的保留策略，且这个差异来自"哪类通道更重要"的预写判断

OutputChannelKind 可以作为工程分组字段，但保留时长不能基于"某类通道天然更重要"的本体决定。

**7.3 禁止：把 cap 数字写成理论时间常数**

`WRITE_EPOCH_HISTORY_CAP` 等常数不能被注释或文档说成"记住 N 个历元，因为这是最优学习窗口"。它们只能被注释为工程近似上界，说明合理初始值的估算方法，并说明后续应由探索历史修正。

**7.4 禁止：把淘汰写成"遗忘"的本体单位**

淘汰旧历元和旧窗口不是"系统在遗忘"的意义上的遗忘。它只是"substrate 中不再持有超出工程上界的低信息量历史记录"的工程清理。高层的"记忆摘要"和"长期遗忘"属于 memory summary / history consolidation 层，不属于本 spec 的 substrate 层。

---

## 8. 引用完整性完整约束表

下表汇总了保留和淘汰操作的引用完整性要求：

| 操作 | 要求 |
|------|------|
| 淘汰写出历元记录 | 已有 pair.write_seq 不变；不回收 write_seq 编号；不修改 pair 内容 |
| 淘汰后果窗口记录 | 已有 pair.window_id 不变；不修改 pair 内容；pair 引用的是稳定 ID 而非 Vec 位置 |
| 淘汰配对记录 | 不破坏历元 ledger 或窗口 Vec；仅影响 duplicate guard（可接受） |
| 追加新历元 | 若超过 CAP，先淘汰最旧的时间窗外历元，再追加 |
| 追加新窗口 | 若超过 CAP，先淘汰最旧的 Closed 窗口，再追加 |
| 追加新配对 | 若超过 CAP，先淘汰最旧的配对记录，再追加 |
| snapshot 写入 | 写入当前活跃历元 + 当前窗口 + 当前配对；不写入已淘汰记录 |
| snapshot 读取 | 从 snapshot 重建时直接使用 window_id（稳定 ID），不假设 Vec 位置 |

---

## 9. 与当前 cleaned substrate 的映射

### 9.1 必须新增的字段

- `ActionConsequenceWindow.window_id: u64` — 稳定窗口 ID
- `ActionConsequenceState.action_consequence_window_seq: AtomicU64`（viewport 级）— 稳定 ID 的发放计数器

### 9.2 必须变更的字段

- `ActionConsequencePair.window_index: u32` → 重命名并语义升级为 `ActionConsequencePair.window_id: u64`
- snapshot 版本号需递增（引入 window_id 字段）

### 9.3 必须新增的常数（lib.rs）

```rust
/// Engineering cap on write_epoch_history Vec length.
/// Knob — not a theoretical learning window. Adjust via exploration history.
pub const WRITE_EPOCH_HISTORY_CAP: usize = 128;

/// Engineering cap on consequence windows Vec length.
/// Knob — not a theoretical consequence count. Adjust via exploration history.
pub const CONSEQUENCE_WINDOW_CAP: usize = 128;

/// Engineering cap on consequence pairs Vec length.
/// Knob — not a theoretical pair count. Adjust via exploration history.
pub const PAIR_HISTORY_CAP: usize = 256;
```

### 9.4 必须新增的 viewport 原子字段

```rust
pub action_consequence_window_seq: AtomicU64,
```

初始值：`0`。在每次 `begin_action_consequence_window` / `push_action_consequence_window` 时自增并赋给新窗口的 `window_id`。

### 9.5 淘汰实现位置

淘汰逻辑应在以下位置触发：

- `apply_action_output_event` 的 Begin 分支（追加历元后检查 epoch cap）
- `begin_action_consequence_window` / `push_action_consequence_window`（追加窗口后检查 window cap）
- `pair_action_consequence` / `try_auto_pair_latest_epoch_to_window`（追加 pair 后检查 pair cap）

每次追加后立即检查并淘汰（不攒批次），以保证 Vec 大小在任何时刻都不超过 cap。

### 9.6 observer / snapshot 受影响边界

- snapshot 需新增对 `window_id` 字段的序列化 / 反序列化
- observer 的 `acq:` 渲染需把 `window_index`（旧 Vec 位置）替换为 `window_id`（稳定 ID）的显示
- 任何测试中使用 `pair.window_index` 的断言需更新为 `pair.window_id`

---

## 10. 开发准入规则

**A10.1**：任何实现不得把淘汰条件写成"result 意义判断"或"重要性排序"，只能用 substrate time 比较和 Vec 长度比较。

**A10.2**：`window_id` 必须从 viewport 级单调原子计数器分配，不得从外部传入，不得使用 `start_t_us` 或 `Vec.len()` 作为 ID（两者均不稳定）。

**A10.3**：三个 cap 常数必须有注释说明它们是工程近似，说明估算来源，说明后续可调，不得注释为"最优记忆窗口"或类似理论断言。

**A10.4**：snapshot 版本号必须在引入 `window_id` 字段时递增，保证旧 snapshot 被安全拒绝而非静默读取错误数据。

**A10.5**：所有现有使用 `pair.window_index` 作为 Vec 位置直接访问 `windows[window_index]` 的代码，必须改为通过 `window_id` 扫描，或在过渡期内维护 `id → position` 的轻量映射。

**A10.6**：任何新测试覆盖 cap 行为时，必须先写失败测试（RED），再实现，再验证（GREEN），遵循项目 TDD 约束。

---

## 11. 收束结论

这份 spec 的核心立场是：

- 写出历元历史、后果窗口历史和配对记录历史，在持续运行的硅基 substrate 中必须有工程上界，但这些上界是 substrate knob，不是"系统真正应该记多少历史"的理论本体
- 淘汰依据只能是 substrate time 结构（历元是否已超出最大延迟窗口）和工程上界（Vec 是否超过 cap），不能是后果意义、重要性地图或对象范围
- 在引入 cap 的同时，必须用稳定 `window_id` 替代脆弱的 Vec 位置索引，保证已有配对记录的引用完整性在任何淘汰操作后仍然成立
- 历元淘汰之后，`pair_action_consequence` 对已淘汰 seq 的验证失败是正确行为，不是错误
- cap 数字的初始值来自工程估算（30 Hz PHOT + 2 s 时间窗 → 几十条即可），后续可由探索历史修正，但这一步修正属于更高层的 memory policy spec

**这份 spec 不回答"系统最终该怎样形成自己的历史保留策略"**——那是更高层的可学习边界问题。这份 spec 只回答：当前 foundation 层的有界历史如何在不引入意义判断、不破坏引用完整性、不把工程近似上升为理论本体的前提下，被合法地维持在一个有限大小之内。
