# 前瞻涌现脚手架 — 软件设计文档 (SDD)

**日期**：2026-06-27
**版本**：v1
**状态**：REVIEW（⑤ 尚未动工，④② 已落地待业主复核）
**前置阅读**：`perturbation-graph-phase-a-to-e-design-2026-06-18.md`、`vocal-language-upgrade-plan-v1.md`

---

## 0. 文档定位

这是**设计先行文档**——记录前瞻涌现脚手架的架构决策、已落地变更、与待实施计划。
不是任务清单。你看完决定继续还是调方向。

**一句话**：让已经在跑的反应式大脑长出前瞻弧——不手写 EFE 规划器，而是搭建**三个最小涌现条件**让前瞻性从关系图里自然结晶出来，镜像音素结晶的成功配方。

---

## 1. 问题陈述

### 1.1 完成度审计结论

2026-06-27 对 bionic_v0 做了 Friston 主动推理 6 组件代码实证审计：

| 组件 | 完成度 | 预期类型 |
|---|---|---|
| C 动作（凝视+发声驱力） | 82% | 真预期 |
| A 生成模型（AR-2×167 + CrossLayer×~50） | 62% | 混合 |
| F 统一目标/驱力 | 45% | partial |
| D 精度/注意力 | 28% | 事后度量 |
| B 知觉推断 | 25% | partial |
| **E 期望自由能/前瞻规划** | **15%** | **几乎为空** |

**整体**：当预测编码生命体 ~60%；当主动推理强义体 ~35%。缺口精准落在 E = 前瞻半边。

### 1.2 两种预期，只兑现一种

系统已有：
- **反应式预期**（真预期）：AR-2 预测器 10ms 前瞻窗、efference copy 在证据到达前注入。动作驱力拿**当下**误差决策。这是 System 1。

系统没有：
- **前瞻式预期**：想象多条未来轨迹、挑最不意外的那条去追。rollout 在代码里存在但全是只读遥测，从未进入决策。这是 System 2 的种子。

### 1.3 根症：一个不是五个

A/B/D/E 的缺口同源 = **"前瞻/精度信号算出来了却没在下一拍喂回预测/决策"**。
诊断标签："建了又丢"——代码里反复出现的同一模式。
这是接线问题，不是缺机件。

---

## 2. 设计哲学

### 2.1 路线 A 纯涌现

业主决策（2026-06-27）：

> "我偏好涌现，但是我们必须保证涌现条件。"

含义：
- ❌ 不手写 EFE 规划器（Hamilton–Jacobi 搜索、tree search、MCTS）
- ❌ 不引入显式目标函数或奖励信号
- ✅ 搭建最小衬底条件让前瞻行为从关系图的信用积累中**自然涌现**
- ✅ 镜像已验证的配方（音素结晶=同一套"变异→选择→保持"闭环）

### 2.2 已验证的先例：音素结晶

2026-06-22 发声多稳态突破——8 个离散音素原型从连续空间结晶出来（最近邻距离 0.367 ≥ 合并半径，稳定复用上百次）。这不是手写的，是三个条件闭合后的涌现：

| 条件 | 发声侧实现 | 前瞻侧对应 |
|---|---|---|
| **变异**（babble） | `VOCAL_BABBLE_FLOOR_NEWBORN=0.42` — 年幼时高探索驱力 | ⑤ 自举地板 |
| **选择**（credit） | `self_match` 闭环 — 自听吻合 → 信用回流到运动指令 | ④ 信用回流 |
| **保持**（retention） | `vocal_memory` — 强化原型沉淀 + 衰减弱原型 | ② 可达性（链持久化→意图沉淀） |

关键洞察：**发声先例并没有"多候选打分"**。音素不是从多候选里选出来的，是变异+选择+保持循环自然收敛的。所以前瞻涌现的最小配方是 3 条不是 6 条。

### 2.3 六条涌现条件（全部审计，三条先修）

完整审计了前瞻涌现的 6 条必要条件。**核心发现：6 条全部 = "破损"（衬底在，线断了），无一 "缺失"。**

| # | 条件 | 状态 | 修复优先级 |
|---|---|---|---|
| ④ | **信用回流** — fired relation 拿到结果信用 | ✅ 已修 | P0 ★ |
| ② | **可达性** — rollout chain 驱动意图 | ✅ 已修 | P0 ★ |
| ⑤ | **自举地板** — 避免 0×x=0 死锁 | 🔲 待修 | P0 ★ |
| ① | 变异（多候选） | 破 | 留给涌现 |
| ③ | 期望意外打分 | 破 | 留给涌现 |
| ⑥ | 想象时间（离线 rollout） | 半破 | 后补加速器 |

**最小配方 = ④ + ② + ⑤**（跨时间形态的变异-选择-保持）。

---

## 3. 架构总览

```
         ┌──────────────────────────────────────────────┐
         │            tick_intention_arbitrator           │
         │                                              │
         │  ① 感知候选 ──→ try_commit()                 │
         │     (anchor → strongest_out_relation → 1 hop) │
         │                                              │
         │  ② 持有场景 ──→ commit_scene()               │
         │     (held_scene_play → 输入解耦回放)           │
         │                                              │
         │  ③ 前瞻 rollout ──→ try_commit()    [新增②]  │
         │     (anchor → rollout_chain(3) → 最远可达)    │
         │                                              │
         └─────────────┬────────────────────────────────┘
                       │
                       ▼
              IntentionState
              ├─ current_target: Endpoint
              ├─ target_rel: RelId
              ├─ intention_pressure: [0,1]
              ├─ commitment_until_us (2s 窗口)
              └─ origin_score (竞争门限 ×1.5)
                       │
                       ▼
              ┌────────────────┐
              │ stamp_mark()   │ → MarkChannel::Intention
              │ → 下游消费者   │   (注意力偏置, 发声驱力场 region C)
              └────────────────┘

  ┌──────────────────────────────────────────────────────────┐
  │              post_hoc_check (Phase C6)       [新增④]     │
  │                                                          │
  │  tick N:  FastPath fires rel_R → stash R                 │
  │  tick N+1: observe anchor_ep → resolve R against anchor  │
  │            predicted `to` == anchor? → +0.01 (reinforce) │
  │            predicted `to` != anchor? → ×0.95 (weaken)    │
  │            + note_post_hoc_outcome() (disagree EMA)      │
  └──────────────────────────────────────────────────────────┘

  ┌──────────────────────────────────────────────────────────┐
  │         anticipation_babble_floor             [计划⑤]    │
  │                                                          │
  │  ANTICIPATION_BABBLE_FLOOR_NEWBORN = 0.30 (探索)         │
  │  ANTICIPATION_BABBLE_FLOOR_MATURE  = 0.05 (精确)         │
  │                                                          │
  │  rollout 每 tick 无条件运行                               │
  │  influence = score.max(floor)                             │
  │  → predict_next 永远算，权重按地板缩放                     │
  │  → 打破 maturity > 0.5 硬门控 (0×x=0)                    │
  └──────────────────────────────────────────────────────────┘
```

---

## 4. 已落地变更：④ 信用回流 (post_hoc_check)

### 4.1 问题

`post_hoc_check` 在 `perturbation_graph.rs:1910` 已完整实现且正确（命中 +0.01 / 未命中 ×0.95），但**全代码零调用 = 死代码**。活路径只调 `note_post_hoc_outcome()` 更新遥测 EMA 不动 `strength`。结果：fired relation 永远拿不到结果信用。这**本身是个 live bug**（反应式路径也坏），不仅仅是前瞻的前置。

旧代码（viewport.rs 意图仲裁器内）用的是**同帧启发式**：在同一个 tick 内算 `sig_distance(&anchor, &predicted)`，但这不是真预期——预测和观察同时发生，没有时间差。

### 4.2 设计

**跨 tick 待决缓冲区模式**（镜像 `held_scene` 的 runtime-only 字段范式）：

```
tick N:   observe anchor → FastPath fires rel_R → set_pending_post_hoc(R)
tick N+1: observe anchor' → take_pending_post_hoc() → post_hoc_check(R, anchor')
                            was rel_R's predicted `to` ≈ anchor'?
                            yes → strength += 0.01
                            no  → strength *= 0.95
```

这是"真预期"诚实的：预测在证据到达**前**承诺，结果在**下一帧**才兑现。

### 4.3 变更清单

**perturbation_graph.rs：**

| 变更 | 位置 | 描述 |
|---|---|---|
| 字段 | `PerturbationGraph` 结构体 | `pending_post_hoc: Option<RelId>` — runtime-only，从不序列化（同 `held_scene`） |
| 初始化 | `Default::default()` + `load` 构造器 | `pending_post_hoc: None` |
| 存取器 | `set_pending_post_hoc()` / `take_pending_post_hoc()` | 一格缓冲区：set 存入，take 取出并清空 |
| 遥测整合 | `post_hoc_check()` 方法内部 | 调用末尾追加 `self.note_post_hoc_outcome(!correct)` — 使 `post_hoc_check` 成为文档所声称的唯一入口 |
| 测试 | `post_hoc_check_feeds_disagree_telemetry_ema` | 验证正确结果不升 disagree EMA，错误结果升 |
| 测试 | `pending_post_hoc_buffer_roundtrips` | 验证一格缓冲区：初空→set→take 返回→再 take 空 |

**viewport.rs：**

| 变更 | 位置 | 描述 |
|---|---|---|
| Phase C6 | 意图仲裁器末尾（~L5911） | 替换旧 sig_distance 启发式 → 跨 tick `post_hoc_check` 调用 |

### 4.4 正确性论证

- **时间因果**：tick N 的预测在 tick N+1 的证据前承诺 → 因果方向正确
- **缓冲区语义**：一格 = 最新预测覆盖旧预测。不需要队列——FastPath 在 30Hz 每 tick 至多 fire 一次
- **遥测整合**：`post_hoc_check` 内部调 `note_post_hoc_outcome` → 单入口，EMA 和 strength 永远同步
- **不影响序列化**：`pending_post_hoc` 是 runtime-only 字段，snapshot 不含（重启后首 tick 无历史待决→正常跳过）

### 4.5 测试

全部 307 测试通过（含新增 2 个），编译无错。

---

## 5. 已落地变更：② 可达性 (rollout_chain → 意图第三候选源)

### 5.1 问题

意图仲裁器只有两个候选源：
1. **感知候选**：anchor → `strongest_out_relation` → 1 hop
2. **持有场景回放**：`held_scene_play` → 输入解耦

缺失的第三源 = **前瞻 rollout**。`quiet_replay_chain` 在发声侧算完即丢（只存 `chain.len()` 当遥测）。`predict_next` 只暖启 servo。关系图有 `strongest_out_relation` 但没人用它做多步前推来驱动意图。

### 5.2 设计

**两件事**：给关系图加多步前推能力，然后把结果喂进意图仲裁器。

#### 5.2.1 `PerturbationGraph::rollout_chain(from, k_steps)`

```rust
pub fn rollout_chain(
    &self,
    from: Endpoint,
    k_steps: usize,
) -> Vec<(Endpoint, RelId, f64)>
```

- 从 `from` 出发，每步走 `strongest_out_relation`（确定性 argmax，不含温度采样）
- 收集 `(to_endpoint, rel_id, strength×expressivity)` 三元组
- 遇环（visited set）或死端（无出边）提前终止
- 环检测用 `HashSet<Endpoint>`（`Endpoint` 已 derive `Hash+Eq`）

**为什么 argmax 不 sample？**

温度采样 (`sample_out_relation`) 依赖 `pg_reasoning_blend`，在年幼图上温度 → 0（argmax）。rollout 是信息通道（"哪里可达"），不是行为输出。探索性通过 ⑤ 自举地板的权重缩放引入，不通过在 rollout 里加噪声。保持 rollout 确定性也让 ④ 信用回流的功过归因干净（你选了 argmax 链的末端，结果好坏清楚归到这条链）。

#### 5.2.2 意图仲裁器第三候选源

```
if silent >= FAST_PATH_SILENT_TICKS {
    // 1) 感知候选 → try_commit (竞争性，需 1.5× 优势才能抢占)
    // 2) 持有场景  → commit_scene (填空，不需优势)
    // 3) 前瞻 rollout → try_commit (填空 + 竞争)  [新增]
}
```

**优先级语义**：

| 候选 | 调用 | 语义 |
|---|---|---|
| 感知 | `try_commit` | 竞争性。空槽直入；占槽须 1.5× 优势 |
| 场景 | `commit_scene` | 填空。只在 commitment_open 时入；不需优势 |
| rollout | `try_commit` | 末位填空。仅当感知与场景都没占槽时入 |

这意味着 rollout 永远不会覆盖已有的感知承诺或场景回放——它只填"什么也没有"的 idle gap。这是对的：前瞻应该在没有更好信息时才接管。

**用 `rollout.last()` 而非 `rollout[0]`**：

- `rollout[0]` = 1 hop = 和感知候选是同一个节点（都来自 `strongest_out_relation(anchor)`）
- `rollout.last()` = K hop = 真正的前瞻——body 还没感知到但**可以到达**的未来状态
- `ROLLOUT_HORIZON = 3`：3 步前瞻。不太投机，也确实超越了即时下一跳

**锁策略**：rollout 和感知候选在同一个 `read()` 锁里计算（零额外锁获取）。

### 5.3 变更清单

**perturbation_graph.rs：**

| 变更 | 位置 | 描述 |
|---|---|---|
| 方法 | `strongest_out_relation` 之后 | `rollout_chain(from, k_steps)` — K 步 argmax 前推 |
| 测试 | `rollout_chain_walks_forward_through_strongest` | 4 节点链 a→b→c→d，验证 3 hop |
| 测试 | `rollout_chain_stops_on_cycle` | a↔b 互指，验证止于 1 hop |
| 测试 | `rollout_chain_stops_on_dead_end` | a→b(无出边)，验证止于 1 hop |
| 测试 | `rollout_chain_empty_when_no_relations` | 空图返回空 Vec |

**viewport.rs：**

| 变更 | 位置 | 描述 |
|---|---|---|
| 常量 | 意图仲裁器 read-lock 上方 | `const ROLLOUT_HORIZON: usize = 3` |
| 计算 | read-lock 内 | `graph.rollout_chain(anchor, ROLLOUT_HORIZON)` |
| 候选 3 | held-scene 块之后 | `commitment_open` → `rollout.last()` → `try_commit` |

### 5.4 正确性论证

- **不增加锁粒度**：rollout 在已有的 `read()` 锁内计算
- **候选优先级**：perception > scene > rollout（由 commitment_open 门控自然排序）
- **环安全**：`HashSet<Endpoint>` + `visited.insert()` 返回 false 时 break
- **空图安全**：`rollout.last()` 返回 None → 不 try_commit
- **K=3 的选择**：3 步 = 在 30Hz orchestrator 下约 100ms 时间跨度的前推。够远到有意义，够近到 credit 链可靠

### 5.5 测试

全部 307 测试通过（含新增 4 个），编译无错。

---

## 6. 待实施：⑤ 自举地板 (anticipation_babble_floor)

### 6.1 问题

当前前瞻（vocal anticipation Stage 3）有硬门控：

```rust
if self.developmental_maturity() > SEQ_ANTICIPATE_MATURITY_FLOOR {  // 0.5
    vocal_sequence.predict_next(last)
}
```

`developmental_maturity()` 需要 `predict_next` 成功才能挣分。`predict_next` 需要 `maturity > 0.5` 才能运行。**0×x = 0 死锁**。

发声侧用 `VOCAL_BABBLE_FLOOR_NEWBORN = 0.42` 破了同类死锁——探索驱力永远 > 0，年幼时高，年老时低（但从不到 0），让自举循环转起来。

### 6.2 设计方案

#### 6.2.1 常量与退火函数

```rust
// lib.rs
pub const ANTICIPATION_BABBLE_FLOOR_NEWBORN: f64 = 0.30;
pub const ANTICIPATION_BABBLE_FLOOR_MATURE: f64 = 0.05;

pub fn anticipation_babble_floor(maturity: f64) -> f64 {
    let m = maturity.clamp(0.0, 1.0);
    (1.0 - m) * ANTICIPATION_BABBLE_FLOOR_NEWBORN
        + m * ANTICIPATION_BABBLE_FLOOR_MATURE
}
```

**参数选择理由**：

| 参数 | 值 | 对标 | 逻辑 |
|---|---|---|---|
| NEWBORN | 0.30 | vocal 0.42 / gaze 1.0 | 前瞻比发声风险高（错误预期→错误动作），起步保守 |
| MATURE | 0.05 | vocal 0.06 / gaze 0.15 | 成熟体基本靠 credit-selected 链，极薄探索层防僵化 |

退火曲线：线性混合（同发声/凝视）。`developmental_maturity()` 从 ~0 开始随通话积累缓升。

#### 6.2.2 门控改造：影响力非存在

核心原则：**predict_next 永远运行，权重按地板缩放**。

改造前（二值门控）：
```rust
if maturity > 0.5 {
    predict_next(last)  // 全有
} else {
    None                // 全无 ← 死锁源
}
```

改造后（连续缩放）：
```rust
let floor = anticipation_babble_floor(maturity);
if let Some((to_idx, edge_score)) = vocal_sequence.predict_next(last) {
    let influence = edge_score.max(floor);
    // use `influence` to weight the warm-start
}
```

`predict_next` 每 tick 都调用。年幼时 `edge_score` 接近 0（序列边还没学出来），`floor` 接管保证 influence > 0。年老时 `edge_score` 远超 `floor`，地板透明。

#### 6.2.3 rollout_chain 同理

rollout_chain 的最远可达节点以 `score` 喂 `try_commit`。年幼图的 `score = strength × expressivity` 接近 0，达不到 `ACTIVE_REASONING_MIN_SCORE = 0.05`。解法：

```rust
let effective_score = score.max(anticipation_babble_floor(maturity));
if effective_score >= ACTIVE_REASONING_MIN_SCORE {
    intention.try_commit(target, Some(rel_id), effective_score, now_us);
}
```

这让年幼体的 rollout 也能进入意图（以地板分数），形成"前瞻 babble"——随机到达的某个前方节点短暂占据意图→body 朝那个方向动→看结果→④ 信用回流告诉它对不对→好的链加强，差的链衰减→② 可达性让加强的链下次更容易被选中→**闭环自举**。

### 6.3 不做什么

- ❌ 不改 `pg_reasoning_blend` / `PG_WALK_MAX_TEMP` 的温度退火方向。当前"越幼越冷"对 rollout 是对的（argmax chain = 确定性前推，好追踪功过）。探索性通过地板分数引入，不通过温度。
- ❌ 不加"多候选"采样。发声先例证明单链 + credit 足以涌现。多候选（①）和期望打分（③）留给成熟后自然涌现或后补。
- ❌ 不加离线 rollout 时间窗。⑥ 想象时间（HEALTH_QUIET 静息期跑 rollout）是加速器，非前置条件。

### 6.4 变更清单（计划）

**lib.rs：**

| 变更 | 描述 |
|---|---|
| 常量 | `ANTICIPATION_BABBLE_FLOOR_NEWBORN = 0.30` / `_MATURE = 0.05` |
| 函数 | `anticipation_babble_floor(maturity) → f64` |
| 测试 | 退火曲线端点 + 单调性 |

**viewport.rs：**

| 变更 | 位置 | 描述 |
|---|---|---|
| Stage 3 | `tick_vocal_drive_field` vocal anticipation | 移除 `maturity > 0.5` 硬门控，换用 `influence = edge_score.max(floor)` |
| 候选 3 | `tick_intention_arbitrator` rollout 段 | `score.max(floor)` 缩放 |

**vocal_sequence.rs：**

| 变更 | 描述 |
|---|---|
| 可能移除 | `SEQ_ANTICIPATE_MATURITY_FLOOR` 常量 — 被 `anticipation_babble_floor` 取代 |

### 6.5 风险

| 风险 | 缓解 |
|---|---|
| 年幼体的 babble rollout 占据意图过频繁，压制感知候选 | rollout 是最低优先级（only fills idle gap）；且 `try_commit` 竞争语义保证感知永远可以 1.5× preempt |
| 地板分数太高导致弱边假装强边 | NEWBORN=0.30 < vocal 0.42，且 ④ credit 会迅速把真弱边压到 ×0.95 衰减 |
| `predict_next` 在年幼图上性能（每 tick 扫序列边） | `predict_next` 是 O(edges_from_cursor)，年幼图边少 → 便宜 |

---

## 7. 三条件闭环拓扑

三个条件不是独立的——它们构成闭环：

```
    ⑤ 自举地板
    ┌──────────┐
    │ floor>0  │ ←── 保证年幼体的 rollout/predict 有非零影响力
    └────┬─────┘
         │ 前瞻 babble → body 动了 → 下一帧有新锚点
         ▼
    ② 可达性
    ┌──────────┐
    │ rollout  │ ←── 前推链持久化为意图候选，body 往那走
    │ → 意图   │
    └────┬─────┘
         │ 意图 target 达成/未达成 → 下一帧检验
         ▼
    ④ 信用回流
    ┌──────────┐
    │ post_hoc │ ←── 对达→+0.01，不对→×0.95，strength 分化
    │ _check   │
    └────┬─────┘
         │ 好链 strength 升 → rollout 下次更可能选中
         │ 差链 strength 降 → rollout 下次更少选中
         ▼
    ② 可达性（再次）
    strongest_out_relation 自然偏向 strength 高的链
    → 信用选择过的好链越来越强 → 越来越被 rollout 选中
    → "前瞻性" 作为涌现现象出现
```

**收敛预期**：类似音素结晶——初期乱走（babble），credit 慢慢分化好链/差链，rollout 逐步偏向好链，身体学会"看到 A 往 B 走是对的"而不是被告知 B 是目标。

---

## 8. 不变量与设计约束

以下不变量在全部三个变更中始终遵守：

1. **无奖励信号**：`post_hoc_check` 的 +0.01/×0.95 是预测误差驱动的信用（predicted `to` vs observed anchor），不是外部奖励
2. **无反传**：所有学习是在线 EMA / 一步信用更新，无批量/epoch
3. **涌现优先**：没有硬编码"目标"——意图来自关系图本身的 credit-accumulated 结构
4. **序列化无染**：所有新字段（`pending_post_hoc`）是 runtime-only，不影响 snapshot 格式
5. **锁纪律**：rollout 在已有 read-lock 内计算，post_hoc 在已有 write-lock 内执行，无新锁
6. **零回归**：空图 / 无关系 / 年幼体 → 所有新路径自然跳过（rollout 返回空 Vec，floor 透明）
7. **温度方向**：rollout 用 argmax（不 sample），探索性通过 ⑤ 地板分数引入——两个旋钮解耦，不互相打架

---

## 9. 观察点与调参指引

### 9.1 活体观察指标

| 指标 | 来源 | 健康信号 |
|---|---|---|
| rollout chain length | `rollout.len()` (可 publish INTERO 轴) | 年幼 0-1，成熟 2-3 |
| post_hoc hit rate | `fast_path_hit_rate()` | 应随通话时间缓升 |
| intention from rollout | 意图来源标记（可加遥测） | 年幼>0（babble floor 在工作），成熟→减少（perception/scene 占主导） |
| relation strength 分化 | `graph_instability()` | 应从 ~0 缓升（分化正在发生） |

### 9.2 调参三旋钮

| 旋钮 | 效果 | 安全范围 |
|---|---|---|
| `ANTICIPATION_BABBLE_FLOOR_NEWBORN` | 年幼体前瞻 babble 强度 | 0.15–0.50 (太低=死锁,太高=乱) |
| `ROLLOUT_HORIZON` | 前推步数 | 2–5 (太短=没有前瞻,太长=投机+环检测开销) |
| `ACTIVE_REASONING_MIN_SCORE` | 意图候选门限 | 0.01–0.10 (与 floor 配合) |

### 9.3 温度方向问题（观察点，不动）

`pg_reasoning_blend` 让温度"越幼越冷（argmax）越熟越暖（sample）"。这对 rollout 本身是对的（年幼时 argmax = 确定性链 = 好追踪功过），但对"多候选变异"(①) 是反的。当 ① 变异日后引入时，可能需要把 rollout 温度和 walk 温度解耦。**现在不动**——观察 ④+②+⑤ 三件套在 argmax 下是否足以涌现基本前瞻。

---

## 10. 实施时序

| 阶段 | 状态 | 内容 | 验证 |
|---|---|---|---|
| ④ 信用回流 | ✅ 落地 | `pending_post_hoc` 缓冲 + `post_hoc_check` 复活 | 2 新测试 pass，全套 307 pass |
| ② 可达性 | ✅ 落地 | `rollout_chain` + 意图第三候选 | 4 新测试 pass，全套 307 pass |
| ⑤ 自举地板 | 🔲 待实施 | `anticipation_babble_floor` + 硬门控→连续缩放 | 退火端点+单调性测试 + 全套 |
| 活体观察 | 🔲 | 上线三件套后观察上述指标 | 至少一次完整通话 |
| ①③⑥ 后续 | 🔲 | 观察三件套涌现效果后决定是否需要 | — |

---

## 11. 风险登记

| # | 风险 | 影响 | 缓解 | 状态 |
|---|---|---|---|---|
| R1 | 收敛到局部极小（所有链都指向同一个强节点） | 前瞻退化为习惯 | ⑤ 地板保底探索 + 未来 ① 多候选 | 观察 |
| R2 | credit 噪声（+0.01/×0.95 不对称）导致系统性偏移 | 所有关系趋弱 | 不对称是刻意的（衰减快于增长 = 宁缺毋滥）；已有 `observe_relation` 持续刷新 | 已分析 |
| R3 | rollout 在密图上 O(K·E) | 性能 | K=3 + `strongest_out_relation` 是 O(edges_from_node) 不是 O(all_edges) | 可控 |
| R4 | 并发会话竞态（concurrent writer） | 工作树合并冲突 | 已知：当前 repo 有并发写者。约束为 commit-no-deploy + 单 dev 分支 | 接受 |
| R5 | ⑤ 地板太激进→年幼体意图抖动 | 感知通道被压制 | rollout 是最低优先级 + try_commit 竞争语义 | 可调旋钮 |

---

## 12. 术语对照表

| 中文 | 英文 / 代码 | 定义 |
|---|---|---|
| 前瞻 | prospection | 想象未来状态并据此行动 |
| 涌现 | emergence | 从低层规则中自发产生的高层模式 |
| 信用回流 | credit assignment / post_hoc_check | 把结果（对/错）归因到产生预测的关系上 |
| 可达性 | reachability / rollout_chain | 从当前状态经关系图可到达的未来状态集合 |
| 自举地板 | bootstrap floor / babble_floor | 防止 0×x=0 死锁的最小非零影响力 |
| 变异→选择→保持 | variation → selection → retention | 进化动力学三要素；涌现的最小闭环 |
| 建了又丢 | built-then-abandoned | bionic_v0 的反复根因模式：机件存在但接线断裂 |
| 真预期 | genuine anticipation | 预测在证据到达前承诺（有时间差） |
| 事后度量 | post-hoc measurement | 预测和观察同帧完成（无时间差→不是真预测） |
