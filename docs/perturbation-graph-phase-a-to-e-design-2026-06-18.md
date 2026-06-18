# perturbation_graph Phase A–E 总设计

**日期**：2026-06-18  
**版本**：v2（重构——撤回"关系→偏好"改名；放大肌肉记忆类比；通俗化未决问题）  
**状态**：REVIEW（动工前提交业主过目）  
**作者**：基于三 pillar architecture audit + 业主反馈

---

## 0. 文档定位

这是 **review 文档**——动工前的总设计。不是任务清单。你看完决定动还是不动、改哪里。

涵盖：架构原则、核心数据结构、关键算法、Phase A–E 详解、持久化策略、风险登记、未决问题（通俗版）。

---

## 1. 核心原则与本次设计的两个基石

### 1.1 三 pillar 审计的统一根因

当前架构是**"perturbation graph 基质 + slot 时代消费者"**。三 pillar 缺口指向同一个核心：

| Pillar | 表面 gap | 根因 |
|---|---|---|
| 神经元 | 只有 node，没有 group / 首类关系 / 标记 | **图是节点的扁平集合，关系不是头等公民** |
| 语言 | vocal substrate 与 perturbation_graph 物理分离 | **两套图，靠 8 维 EMA 桥接，认知和语言不同基质** |
| 推理 | 消费者把图压成一个标量 `most_bound` | **图被建出来但没人关系性读它，目标/意图缺位** |

所有改动都围绕一件事：**让图真正"被读"成图，而不是被读成"最亲的那个"**。

### 1.2 业主反馈整合（v2 重要修正）

#### 关于 Q1: 软成员关系 [接受]

> 每个 GroupMember 的权重由 sig 距离 → Gaussian → EMA。

成员关系是**流动的**——一个节点可以"渐渐淡出 group"或"渐渐被吸入"，不需要二元决策。

#### 关于 Q2: 关系名字保留 [v1 改名撤回]

业主原话：

> "关系本身确实是关系。它是使用 tier 0 轴来描述两个事物之间的关联。关联本身有详细和强弱等。所以被我描述成偏好。但是在真理而言，它就是关系。"

**v2 修正**：
- 不改名——代码里仍叫 `Relation` / `RelationKind` / `RelId`
- 但 v1 描述的**内涵**完全保留：
  - 关系的强度由 **tier-0 轴对齐数量**决定（一段 Relation 在多少个 tier-0 轴上"对得上" → 这段关系就有多详细/多强）
  - **理论上无限种类**：RelationKind 是动态发现的 cluster id，不是 enum
  - **理论上无限深度**：一段关系可指向另一段关系，递归无上限（工程加 cycle 保护）
  - 当认知足够 + 强化通道沉淀 → 涌现"战略级大事预测"（沿关系图多步前推）

业主校正的本质：**关系不是偏置（preference 这种 RL 术语），它就是"两段扰动之间用 tier-0 轴描述的关联"——这是认识论层面的关系**。代码命名跟着这个真理走。

#### 关于 Q3: 肌肉记忆是默认意识 [v1 远不到位，重写]

业主原话：

> "类似于人的肌肉记忆，在相同的原则下设计。这个肌肉记忆比我那些苍白的描述要强有力。事实上**绝大多数部分的事情都是随着肌肉记忆+被动推理来处理的，甚至被朋友随口问的一些日常问题**。连推理都是回答后再去考虑刚刚发生的事情是否真的经过大脑。"

这一段是整个架构观的质变。v1 文档把 FastPath 当 perf 优化（缓存层），错了。正确观点：

**FastPath = 默认认知基质，覆盖绝大多数行为。慢推理是例外、是事后、不一定每次触发。**

这对应 Kahneman 的 System 1 / System 2，但更激进——业主指出连"反思"本身都常常是**事后**才发生（你回答完朋友的问题，几秒后才想起"刚才我说的对吗"），甚至不一定真的反思过。

这有四个架构含义：

1. **查询顺序倒过来**：感知到状态 → **先查 FastPath**（有响应？有就走，结束）→ 没有才走慢图遍历 + 主动推理
2. **慢推理是 exception path**，不是 default path。绝大多数 tick 不应该启动主动推理
3. **事后审计是异步、可选**：FastPath 响应已经发出后，系统**可能**调度一个慢检查（如果注意力允许），更新或矫正 FastPath 条目。注意"可能"——不是每次
4. **小脑反射总线 = FastPath 的高频查询通道**：v1 把 CerebellarReflexBus 和 FastPath 分两个 Phase 做了——其实它们是同一个东西的两个面。FastPath 是数据，反射总线是它的高频驱动器

**业主级别隐含的能量预算原则**：cortex 慢推理贵，反射/习惯便宜。系统大部分时间应该跑便宜的，只在需要时贵的才被唤起。

### 1.3 三个不变量（Phase A–E 始终遵守）

1. **关系可指向任意 Endpoint**（Node / Group / Relation），递归无上限，靠 visited set 防 cycle
2. **强度永远派生自 tier-0 表达力 × EMA 累计**，不是硬编码 α（让"千丝万缕"自然涌现）
3. **FastPath 是默认认知基质**——任何查询先查 FastPath，miss 才走全图。FastPath 的存在改变**性能**也定义**默认行为**

---

## 2. 核心数据结构

### 2.1 Endpoint（无限递归）

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Node(NodeId),       // 单个 perturbation patch
    Group(GroupId),     // 一团 patch 的软集合
    Relation(RelId),    // 一段关系本身（meta-relation）
}
```

无深度上限。任何遍历必须传 `visited: HashSet<Endpoint>` 防 cycle，工程上加 `max_depth` 配置（默认 8 用于推理路径，32 用于离线 merge）。

### 2.2 NodeGroup（软成员）

```rust
pub struct GroupMember {
    pub node_id: NodeId,
    /// 派生自 sig_distance(node.sig, group.centroid_sig) → gaussian → EMA。
    /// distance 越大 → weight 越小；node 长期不活跃 → weight 自然衰减。
    pub weight: f64,
    pub last_refreshed_t_us: u64,
}

pub struct NodeGroup {
    pub id: GroupId,
    pub centroid_sig: [f64; PG_NODE_SIG_DIM],
    pub members: Vec<GroupMember>,
    pub region: RegionKey,
    /// 软 bind_strength = Σ(member.weight × member.bind_strength) / Σ(member.weight)
    pub bind_strength: f64,
    pub formed_t_us: u64,
    pub last_active_t_us: u64,
}

pub type GroupId = u32;
```

Centroid 也是 EMA：每次成员被刷新，centroid 按 member.weight 拉。Group 会**漂移**——和"细胞集群在皮层柱内位置不固定"对得上。

### 2.3 RegionKey（区域运算符）

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegionKey {
    Spatial(u8, u8),         // 视觉空间桶
    TemporalWindow(u64),     // 时间窗
    NestSubtree(NodeId),     // Nest 关系子树根
    CoOccurCluster(u32),     // 在线发现的 cooccur 簇
}
```

一个 node 可同时是多个 group 成员（高 weight 在视觉 spatial group，中 weight 在时间窗 group，低 weight 在 cooccur cluster）。

### 2.4 Relation（首类关系，v1 的 Preference 改名回归）

```rust
pub struct Relation {
    pub id: RelId,
    pub from: Endpoint,
    pub to: Endpoint,
    pub kind: RelationKind,
    /// EMA 累计，在 [0,1]。这是"关系成立的程度"。
    pub strength: f64,
    /// 在 [0,1]，源和目标在多少 tier-0 轴上对齐。
    /// 决定本条 relation 向上携带的 credit。
    /// 业主校正核心：关系的"详细程度" = 轴对齐数量。
    pub expressivity: f64,
    pub elig: f64,
    pub last_t_us: u64,
    pub last_axes_engaged: u8, // 多少个 tier-0 轴在本次激活点燃
}

pub type RelId = u32;
```

**strength 和 expressivity 分离**：一段关系可以被反复观察到（高 strength）但只在 2 个轴上对齐（低 expressivity）；也可以只观察过几次但在 22 个轴上完全对齐。下游推理优先用 expressivity 高的关系做"大事预测"。

### 2.5 RelationKind（开放词汇）

```rust
pub struct RelationKind {
    /// 由在线聚类发现的 id。
    /// 0..4 保留给 primitive 类型（Cooccur=0, Nest=1, Causal=2, CrossModal=3），
    /// 5..u32::MAX 给涌现 kind。kind 自身有 strength + age，会被淘汰。
    pub cluster_id: u32,
}
```

涌现 kind 的发现机制：当一组 relation 在 (from-sig, to-sig, dt, modality) 上聚类显著且未被现有 kind 覆盖时，分配新 cluster_id。淘汰策略：长期无新 relation 加入该 cluster → demote 回 primitive。

### 2.6 FastPath（肌肉记忆——v2 重新定位为默认认知基质）

**v1 把 FastPath 当性能优化层（缓存）。v2 修正：FastPath 是默认认知的存放处，绝大多数行为通过它发生。**

```rust
pub struct FastPath {
    pub rel_id: RelId,
    pub promoted_t_us: u64,
    pub usage_count: u64,
    /// 上次实际驱动行为的时间。用于 sparsity-weighted trigger。
    pub last_fired_t_us: u64,
}

pub struct FastPathIndex {
    /// 主索引：(from, to) → 强化的 relation
    by_endpoints: HashMap<(Endpoint, Endpoint), FastPath>,
    /// 反向索引：from → 它的所有 FastPath 出口
    by_from: HashMap<Endpoint, SmallVec<[RelId; 8]>>,
}
```

**Promotion 规则**：
- `relation.strength > FAST_PATH_PROMOTE_TH`（默认 0.8）
- 持续超过 `FAST_PATH_PROMOTE_DURATION`（默认 5 分钟实时）
- `expressivity > FAST_PATH_EXPRESSIVITY_MIN`（默认 0.4）

**Demotion 规则**（hysteresis）：
- `strength < FAST_PATH_DEMOTE_TH`（默认 0.5）
- 或 `expressivity < 0.2`（轴对齐瓦解）

**核心查询语义**：

```rust
fn query_response(&self, from: Endpoint, ctx: &Context) -> Option<Response> {
    // 1. 先查 FastPath（默认/肌肉记忆路径）
    if let Some(rel) = self.fast_paths.lookup(from, ctx) {
        return Some(execute(rel));  // 大多数情况到这里就结束了
    }
    // 2. FastPath 沉默 → 走慢图遍历 + 主动推理
    self.slow_deliberation(from, ctx)
}
```

**事后审计（可选）**：当注意力 budget 允许，调度一个 async slow_check 重新评估 FastPath 刚才的响应是否合理；不合理就降级或纠正。

**这正是业主所说的**："连推理都是回答后再去考虑刚刚发生的事情是否真的经过大脑"——FastPath 已经响应，slow 反思是事后、可选、不一定触发。

### 2.7 MarkRef（指向/标记操作符）

```rust
pub enum MarkRef {
    Node(NodeId),
    Group(GroupId),
    Relation(RelId),
}

pub struct Mark {
    pub target: MarkRef,
    pub stamped_by: MarkChannel,
    pub strength: f64,
    pub stamped_t_us: u64,
}

pub enum MarkChannel { Attention, Curriculum, DualDrive, Intention }
```

每个 channel 在任一时刻最多一个 active mark（避免多 writer 各拍各的）。

`BindingTargetRef::PerturbationNode { node_id }` 升级为 `BindingTargetRef::PerturbationMark(MarkRef)`——binding 可绑节点、群、关系。

### 2.8 PerturbationGraph（聚合）

```rust
pub struct PerturbationGraph {
    pub nodes: Vec<PerturbationNode>,
    pub groups: Vec<NodeGroup>,
    pub relations: Vec<Relation>,
    pub kinds: Vec<RelationKind>,
    pub fast_paths: FastPathIndex,
    pub marks: SmallVec<[Mark; 8]>,

    pub next_node_id: u32,
    pub next_group_id: u32,
    pub next_rel_id: u32,
    pub next_kind_id: u32,
}
```

容量上限均为软上限 + eviction defence：
- `PG_NODE_CAP = 8192`
- `PG_GROUP_CAP = 1024`
- `PG_REL_CAP = 131_072`（"千丝万缕"的硬储备）
- `PG_KIND_CAP = 256`
- `PG_FAST_PATH_CAP = 2048`

按 RelationKind 软配额（每 kind 至少 5% 池容量），防止视觉爆发挤掉语言学到的关系。

---

## 3. 关键算法

### 3.1 Group formation（区域内 perturbation 聚合）

每 tick（vocal 50Hz / vision 30Hz）做一次轻量聚合：

```
for each active node n:
    for each candidate region r in {spatial_bucket(n), temporal_window(now), n's nest_root}:
        let g = nearest_group(r, n.sig, max_dist=GROUP_FORM_DIST)
        if let Some(g) = g:
            soft_admit(g, n, weight = gaussian(sig_distance(n.sig, g.centroid_sig)))
            ema_update_centroid(g, n.sig, weight)
        else if active_nodes_in_region(r) >= GROUP_FORM_MIN_NODES:
            create_group(r, members = active_nodes_in_region(r))
```

`soft_admit` 不替换硬成员资格——它只更新该 (group, node) 对的 weight EMA。

### 3.2 Soft membership weight EMA

```
let d = sig_distance(node.sig, group.centroid_sig);
let target_weight = gaussian(d, sigma=GROUP_MEMBERSHIP_SIGMA);
member.weight = (1 - alpha) * member.weight + alpha * target_weight;
member.last_refreshed_t_us = now_us;

// 长期不活的成员 weight 自然衰减
if now_us - member.last_refreshed_t_us > GROUP_MEMBER_DECAY_US:
    member.weight *= decay_factor;
```

### 3.3 Relation expressivity 计算

每次 `observe_relation(from, to, kind, now_us)`：

```
let from_sig = endpoint_sig(from); // 如果 from 是 Group/Relation，递归取 centroid
let to_sig = endpoint_sig(to);

let mut engaged = 0;
let mut similarity = 0.0;
for i in 0..PG_NODE_SIG_DIM:
    let s = 1.0 - (from_sig[i] - to_sig[i]).abs();
    if s > EXPRESSIVITY_AXIS_TH:  // 默认 0.5
        engaged += 1;
        similarity += s;

let new_expressivity = engaged as f64 / PG_NODE_SIG_DIM as f64;
relation.expressivity = (1 - α_expr) * relation.expressivity + α_expr * new_expressivity;
relation.last_axes_engaged = engaged as u8;
```

**用于下游 credit**：推理沿关系走一步时，credit 按 `strength × expressivity` 加权。

### 3.4 FastPath 查询优先（v2 核心：默认认知通道）

```rust
impl PerturbationGraph {
    /// 默认响应路径：先查 FastPath（肌肉记忆），miss 才考虑慢推理。
    pub fn default_response(
        &self,
        from: Endpoint,
        ctx: &QueryContext,
    ) -> ResponseOutcome {
        // 第一步：FastPath 查找
        if let Some(rel_id) = self.fast_paths.lookup(from) {
            let rel = &self.relations[index_of(rel_id)];
            // 检查上下文是否兼容（kind 匹配、目标活跃等）
            if self.fast_path_applicable(rel, ctx) {
                self.note_usage(rel_id, now_us());
                return ResponseOutcome::FastPath(rel.to);
            }
        }
        // 第二步：FastPath 沉默或不适用 → 落回慢图
        ResponseOutcome::NeedsDeliberation
    }

    /// 调用者收到 NeedsDeliberation 后，可选地启动慢推理。
    /// 这一步**不是每次都跑**——主动推理由独立的 IntentionState 调度。
    pub fn slow_walk(
        &self,
        from: Endpoint,
        max_depth: usize,
        visited: &mut HashSet<Endpoint>,
    ) -> Vec<RelId> { ... }

    /// 事后审计：FastPath 响应已经发出，系统在 idle 时回顾。
    pub fn post_hoc_check(&mut self, fired_rel: RelId, observed_outcome: Endpoint) {
        let rel = &mut self.relations[index_of(fired_rel)];
        let predicted_to = rel.to;
        let correct = endpoint_overlap(predicted_to, observed_outcome) > POST_HOC_AGREE_TH;
        if correct {
            rel.strength = (rel.strength + 0.01).min(1.0); // 强化
        } else {
            rel.strength *= 0.95; // 弱化
            // 如果 demote 阈值穿越，FastPath 会自动撤出
        }
    }
}
```

**业主原话兑现**：
- "绝大多数事情都是肌肉记忆处理的" → 主路径只走 FastPath，慢路径默认不启
- "推理是回答后才发生" → `post_hoc_check` 是 idle 时调度，不阻塞响应
- "甚至不一定真的经过大脑" → `post_hoc_check` 调用频率受 `attention_budget` 节制，不是每次都跑

### 3.5 FastPath promote / demote（hysteresis）

每 1Hz 扫一遍 relations：

```
for each relation r:
    let cur_state = fast_paths.contains((r.from, r.to));
    let above_promote =
        r.strength > FAST_PATH_PROMOTE_TH &&
        (now_us - r.first_above_th_t_us) > FAST_PATH_PROMOTE_DURATION &&
        r.expressivity > FAST_PATH_EXPRESSIVITY_MIN;
    let below_demote =
        r.strength < FAST_PATH_DEMOTE_TH ||
        r.expressivity < FAST_PATH_DEMOTE_EXPRESSIVITY;

    match (cur_state, above_promote, below_demote) {
        (false, true, _)  => fast_paths.insert(r.id),
        (true, _, true)   => fast_paths.remove(r.id),
        _ => /* no change */
    }
```

### 3.6 Endpoint 循环保护遍历

任何沿关系图走的算法都必须：

```rust
fn traverse(start: Endpoint, max_depth: usize, visit: impl FnMut(Endpoint, usize)) {
    let mut stack: Vec<(Endpoint, usize)> = vec![(start, 0)];
    let mut visited: HashSet<Endpoint> = HashSet::with_capacity(max_depth * 8);
    while let Some((ep, d)) = stack.pop() {
        if d > max_depth || !visited.insert(ep) { continue; }
        visit(ep, d);
        // push out-relations as next frames
    }
}
```

推理路径默认 `max_depth=8`；merge 时 `max_depth=32`。

### 3.7 跨谱系 merge

1. **Node** 匹配（已有 sig 距离逻辑）
2. **PreferenceKind RelationKind prototype** 对齐（按 kind 的原型 signature 距离匹配）
3. **Relation** 端点重映射 + 取 max(strength × expressivity)
4. **Group** centroid 距离 + region key 兼容时合并，成员并集，weight 取 max
5. **FastPath 不 merge**——本地优化，每次 boot 自动重建

---

## 4. Phase 详解（依赖图）

### 4.1 依赖图

```
A1 NodeGroup ──┐
A2 RegionKey ──┤
A3 Relation + Endpoint  ── ┬── A6 vocal_target_recall → PerturbationNode
A4 RelationKind discovery ─┤
A5 FastPath (默认通道) ────┤
A7 MarkRef ────────────────┘
   │
   ├── A8 vocal_sequence → Relation (kind::Sequence)
   ├── A9 Persistence v62 + merge
   ↓
B1 LanguageBindingState → graph relations
B2 global_relations/reasoning → graph
B3 Consumers group-aware（read FastPath first）
   ↓
C1 IntentionState ── C2 endogenous attention bias ── C3 active_reasoning gate（FastPath 沉默触发）
                  └── C4 salience-weighted replay ── C5 ReasoningFragment predictor (post-hoc check)
   ↓
D1 vocal_drive_field region C ── D2 quiet-replay → drive ── D3 delete SymbolSeq
   ↓
E1 open kind vocab ── E2 vision_tier0 densification ── E3 tick rate split ── E4 反射总线 = FastPath 高频驱动
```

### 4.2 Phase A: Substrate Unification（4-6 周）

**目标**：让"图"真的是图——groups + relations + fast paths + marks + 统一 vocal/visual 节点。

| 步骤 | 内容 | 工作量 |
|---|---|---|
| A1 | NodeGroup 数据结构 + add/refresh/nearest/active group + soft membership | 3-4d |
| A2 | RegionKey + region_members 查询 + group formation pass | 2-3d |
| A3 | Relation 升首类（替换 PerturbationEdge）+ Endpoint enum + 循环保护遍历 | 4-5d |
| A4 | RelationKind 数据结构 + 在线 cluster 发现 + primitive id 0..4 | 3-4d |
| A5 | FastPath 索引 + 查询语义（**默认通道，先查 FastPath 再图**）+ promote/demote | 4-5d |
| A6 | vocal_target_recall RecallEntry 迁进 PerturbationNode | 5-7d |
| A7 | MarkRef + Mark + 4 channel 拥有规则 | 2d |
| A8 | vocal_sequence SeqEdge → Relation(kind::Sequence) + DTW seq_fit 改图行走 | 5-7d |
| A9 | perturbation_graph.bin v2 + cross-lineage merge 重写 | 3-4d |
| A10 | SNAPSHOT_VERSION 61→62 + 测试 | 1-2d |

**v2 关键调整**：A5 比 v1 的 FastPath 更核心——它是默认响应路径的入口，不是缓存。每个 query API 都先查 FastPath。

### 4.3 Phase B: Consumer Rewire（2-4 周）

| 步骤 | 内容 | 工作量 |
|---|---|---|
| B1 | 删 LanguageBindingState，所有 binding 改成 graph 里的 Relation(kind=Binding) | 5-7d |
| B2 | refresh_global_relations / refresh_global_reasoning 改读 perturbation_graph relations | 4-5d |
| B3 | tick_perturbation_intero 改读 most_bound_group + FastPath 当前 active 输出；tick_dual_drive 同步 | 2d |
| B4 | observer.rs 渲染加 group / relation / fast_path 摘要 | 1-2d |
| B5 | training_pool.mjs Gate 加 "relations > N" 或 "fast_paths > M" 为成长证据 | 1d |

### 4.4 Phase C: Active Reasoning（4-6 周）

**v2 关键调整**：主动推理由"FastPath 沉默"触发，不是无条件每 tick 跑。

| 步骤 | 内容 | 工作量 |
|---|---|---|
| C1 | 新模块 `intention.rs`：IntentionState { current_target, target_rel, intention_pressure, persistence_us, commitment_until_us, satisfaction } | 5-7d |
| C2 | tick_intention_arbitrator：FastPath 输出低置信或沉默 → 启 active 推理 → 选 target | 4-5d |
| C3 | refresh_global_attention 加 intention bias（在 stimulus-driven 项之前） | 2d |
| C4 | active_reasoning gate：FastPath 有适用响应 → 跳过本 tick gap_pressure 计算 | 1d |
| C5 | tick_quiet_history_replay 改 salience-weighted 抽样 perturbation_graph 节点 | 3d |
| C6 | ReasoningFragment predictor + post_hoc_check：FastPath 响应已发，事后异步审计 | 7-10d |

### 4.5 Phase D: 主动发音表达（1-2 周）

| 步骤 | 内容 | 工作量 |
|---|---|---|
| D1 | vocal_drive_field 加 region C：FastPath 里有 visual→vocal 的强 relation 时直接驱动 servo | 3-4d |
| D2 | tick_vocal_sequence_quiet_replay：第一跳直接进 drive field | 2d |
| D3 | 删 PatternKind::SymbolSeq + mark_disturbance.observe_bytes | 2-3d |
| D4 | sparsity-weighted trigger：region C 看"该 FastPath 最近多久没用过"加权——避免 monotone babble | 2d |

### 4.6 Phase E: 优化与密化（2-4 周）

| 步骤 | 内容 | 工作量 |
|---|---|---|
| E1 | RelationKind::Emergent 完成在线 cluster 发现 + 淘汰 | 5-7d |
| E2 | vision_tier0 加 Nest / Causal / CrossModal 边发射 | 3-4d |
| E3 | body.rs tick chain 拆 fast-tier 200Hz / slow-tier 20Hz | 3-4d |
| E4 | 反射总线 = FastPath 高频驱动器（不是独立模块，是 200Hz 频率 + 强制 FastPath-only 查询的 tick） | 3-4d |

---

## 5. 持久化与 Snapshot

### 5.1 SNAPSHOT_VERSION 61 → 62（Phase A 结束 bump）

破坏性变更，谱系成熟度重置。这是 Phase A 的硬代价。

### 5.2 perturbation_graph.bin sidecar v1 → v2

- 自版本（PGRA magic, version=2）
- v1 → v2 拒绝读取（nursery 重启重建，访客 session 接受 cold start）
- v2 加入 groups / relations / kinds / fast_paths / marks 块
- 每块固定 stride + 长度前缀 + 末尾 CRC

满载估计 ≤ 10 MB。

### 5.3 跨谱系 merge_perturbation_graph 重写

参见 §3.7。

---

## 6. 可观察性

### 6.1 新轴

- `INTERO_INTENTION_PRESSURE` — 主动推理当前压力
- `INTERO_GRAPH_DENSITY` — relations.len() / nodes.len()
- `INTERO_FAST_PATH_HIT_RATE` — FastPath 命中率（默认响应占比）
- `INTERO_GROUP_BIND_TOP` — most_bound_group().bind_strength
- `INTERO_POST_HOC_DISAGREE_RATE` — 事后审计推翻 FastPath 的比例

最后一项最有诊断价值：如果一直 0，说明肌肉记忆从不出错（或者从不被审计）；如果太高，说明 FastPath promote 标准太松。

### 6.2 测试 fixture

- Group 形成 / 软成员衰减 / centroid 漂移：8 测试
- Relation 创建 / 端点解引用 / 循环保护：6 测试
- FastPath promote/demote hysteresis：4 测试
- **FastPath-first 查询语义**：5 测试（关键：默认走 FastPath，sub-step 退到图）
- **post_hoc_check 异步审计**：3 测试
- RelationKind 在线发现：4 测试
- MarkRef 通道 ownership：3 测试
- 跨谱系 merge 完整往返：3 测试

---

## 7. 未决问题（通俗版，请 review 时回答）

v1 七个问题写得太工程化你看不懂——重写。每题给具体场景 + 选项。你选一个就行，不需要懂细节。

### Q-A：Group 的"中心样子"用谁的？

**场景**：系统看到三只猫聚在一起，它把它们归成一个 "猫群组"。现在系统问自己："这个猫群组的中心样子是什么颜色？"

两种算法：
- **(a) 平均**：三只猫颜色的平均值（一只灰、一只黑、一只白 → 中灰）
- **(b) 最近活的那只**：三只猫里最近被看清楚的那只（比如刚抓住注意力的黑猫 → 黑）

**含义**：选 (a) 稳定但有时模糊；选 (b) 灵敏但抖动。

**默认建议**：(a) + 5% 混 (b) 风味，最稳定。

请选：(a) / (b) / 默认 / 其他描述

---

### Q-B：FastPath（肌肉记忆）要不要存到硬盘？

**场景**：你今天用 5 分钟把一个动作练成了肌肉记忆。明天 AI 重启了。

两种做法：
- **(a) 存硬盘**：明天启动直接用，记忆延续。约 50KB 文件
- **(b) 不存**：每次启动从零，根据图里的 strength × duration 自动重新发现

**含义**：肌肉记忆按你的描述是"很稳固的"，应该存。但 (b) 让冷启动更纯净——每次都是 fresh 状态，没有沉积偏差。

**默认建议**：(a) 存

请选：(a) / (b)

---

### Q-C：注意力 mark 多久后自动撤回？

**场景**：你看到一个东西，AI "标记"它说"我正在注意这个"。然后你看别的去了，但 AI 没主动撤回这个标记。

问题：AI 多久后自动忘掉这个"我在注意 X"？

- **(a) 30 秒**（默认）
- **(b) 5 秒**（短）
- **(c) 5 分钟**（长，像深思）
- **(d) 不自动撤回，必须明确改变注意力才换**

**默认建议**：(a) 30 秒

请选：(a) / (b) / (c) / (d)

---

### Q-D：系统发现的新关系类型要不要有名字？

**场景**：系统自己发现一种新关系（比如"大物体包含小物体"），但它只是个编号 `kind_47`。

- **(a) 只显示编号**（k47, k48...）：纯涌现，不解释
- **(b) 编号 + 一个例子**（k47 = "你 → 你的手机"）：dashboard 上有个参考样子让人能猜出是什么意思

**含义**：(b) 不影响系统行为，纯粹是 dashboard 显示。

**默认建议**：(b)，但只是 dashboard hint，不影响系统

请选：(a) / (b)

---

### Q-E：vocal 节点合并进 perturbation_graph 时，要不要保留旧的作为备份？

**场景**：把 vocal 系统融进 perturbation_graph 是大改。如果做错了，怎么回？

- **(a) 干净切换**：旧的 vocal_target_recall 直接删，回退靠 git revert
- **(b) 双栈并存一段时间**：新旧都跑，新的稳定后再删旧的

**含义**：(a) 干净但风险大；(b) 保险但代码复杂（之前 slot cutover 走过双栈，有教训：双栈期间常有诡异 bug 因为两套数据漂移）

**默认建议**：(a) 干净切换，相信测试覆盖

请选：(a) / (b)

---

### Q-F：关系池的容量要多大？

**场景**：人脑每个神经元约有 1 万个关系。我们的节点 8192 个，理论应有 8 千万关系。

- **(a) 13 万关系**（默认，约 6 MB 内存）：当前折衷
- **(b) 100 万关系**（约 50 MB 内存）：宽裕些，"千丝万缕" 更接近
- **(c) 800 万关系**（约 400 MB 内存）：接近生物比例

**含义**：(c) 让涌现更可能；但内存膨胀对部署有压力（服务器还行，但同时跑很多访客 session 就吃紧）。

**默认建议**：(b) 100 万——比默认宽，又不挤内存

请选：(a) / (b) / (c)

---

### Q-G：关系图能链式追多深？

**场景**：你看到 A → A 引发 B → B 导致 C → C 让人哭。系统沿关系往后想，能想几步？

- **(a) 8 步**（默认）：够"我 → 看 → 它做 → 后果 → 后果的后果"
- **(b) 16 步**：更深的推理链，但每次推理慢 2 倍、内存翻倍
- **(c) 自适应**：根据当前 IntentionState 的"我在想多大的事" 动态调

**默认建议**：(a) 8 步，先看真实推理需要多深再调

请选：(a) / (b) / (c)

---

## 8. 风险登记

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| A3 Relation 替换 PerturbationEdge 破坏当前测试 | 高 | 大 | A3 后立即全套测试，每个错都修；ParallelEdge 兼容层撤掉前严格 review |
| A6 vocal 节点合并打破 DTW seq_fit | 中 | 大 | 测试保留 "DTW alignment beats per-frame fit" invariant；真实通话验证 efficacy 不退 |
| Phase A 完成后 nursery 谱系 maturity 归零 | 高 | 中 | 业主接受过一次，文档标记 "Phase A 部署" 时间锚点 |
| Group 软成员 EMA 参数不当导致抖动或粘滞 | 中 | 中 | 三场景测试覆盖：outlier 进出 / centroid 漂移 / 长期不活淘汰 |
| FastPath promote 阈值不当导致抖动 | 中 | 中 | hysteresis（0.8 / 0.5）+ 5 分钟 duration + 监控抖动频率 |
| **FastPath-first 查询模型导致系统僵硬**（只跑肌肉记忆，新情景无应对） | 中 | 大 | Q-A/Q-B 决定后 active_reasoning gate（Phase C）必须落地，否则 cold-start 没有任何应对路径 |
| IntentionState 被 stimulus 劫持 | 中 | 大 | commitment 最少 2 秒，新 intention 必须 strictly dominate 才能抢占 |
| 主动发音对着同一节点 monotone | 高 | 中 | sparsity-weighted trigger（D4） |
| RelationKind::Emergent 发现 id 爆炸 | 中 | 中 | PG_KIND_CAP=256 + 弱 kind 淘汰 + 强 novelty threshold |
| 跨谱系 merge 在 kind 对齐时错配 | 中 | 中 | kind prototype signature 距离阈值（0.15）+ merge log 记录对齐数量 |
| 实际通话视觉数据让 REL_CAP 也饱和 | 中 | 小 | 监控 relations.len() / cap 比例；超过 80% 报警；动态扩容是 Phase E |

---

## 9. 工程承诺

1. **Phase A 是不可分整块**——4-6 周。期间服务器跑的是当前架构（已部署的 d109888 + a3f74d6 + 1eeae5e）
2. **每个 Phase 结束部署一次**，跑 24-48 小时观察图结构
3. **每个 Phase 后一份"Phase 完成报告"**——形态、指标、回归、下一步决策
4. **遇到设计选择超出本文档预设**，停下来问你

---

## 10. Review 检查清单（请你过一遍）

- [ ] **架构观重构**：FastPath = 肌肉记忆 = 默认响应路径（不是性能优化）——接受？
- [ ] 关系名字保留"Relation"——确认？
- [ ] 软成员关系（按 centroid 距离自调）——接受？
- [ ] Phase A 谱系成熟度归零——接受？
- [ ] Q-A 到 Q-G 七个选择题——回答
- [ ] 优先级顺序 A→B→C→D→E 严格按依赖——接受？还是想先做 D 看现象？
- [ ] Phase A 单 PR 整块（4-6 周）——接受？还是拆成 3 个 PR？

---

## 附录 A: 关键常量初值表

```rust
// Capacity caps
pub const PG_NODE_CAP: usize = 8192;
pub const PG_GROUP_CAP: usize = 1024;
pub const PG_REL_CAP: usize = 131_072;  // Q-F 选项 (a)；可调到 (b) 1_048_576 或 (c) 8_388_608
pub const PG_KIND_CAP: usize = 256;
pub const PG_FAST_PATH_CAP: usize = 2048;

// Soft per-kind quota
pub const PG_PER_KIND_FLOOR_FRAC: f64 = 0.05;

// Group formation
pub const GROUP_FORM_DIST: f64 = 0.20;
pub const GROUP_FORM_MIN_NODES: usize = 3;
pub const GROUP_MEMBERSHIP_SIGMA: f64 = 0.15;
pub const GROUP_MEMBER_DECAY_US: u64 = 30_000_000;

// Relation
pub const EXPRESSIVITY_AXIS_TH: f64 = 0.5;
pub const REL_ALPHA: f64 = 0.20;
pub const ALPHA_EXPRESSIVITY: f64 = 0.10;

// FastPath
pub const FAST_PATH_PROMOTE_TH: f64 = 0.80;
pub const FAST_PATH_DEMOTE_TH: f64 = 0.50;
pub const FAST_PATH_PROMOTE_DURATION_US: u64 = 300_000_000;
pub const FAST_PATH_EXPRESSIVITY_MIN: f64 = 0.40;
pub const FAST_PATH_DEMOTE_EXPRESSIVITY: f64 = 0.20;
pub const POST_HOC_AGREE_TH: f64 = 0.7;
pub const POST_HOC_ATTENTION_BUDGET: f64 = 0.1; // 10% 的 idle 时间用来事后审计

// Traversal
pub const RELATION_TRAVERSAL_MAX_DEPTH: usize = 8;  // Q-G 选项 (a)
pub const MERGE_TRAVERSAL_MAX_DEPTH: usize = 32;

// Intention
pub const INTENTION_COMMITMENT_MIN_US: u64 = 2_000_000;
pub const INTENTION_PREEMPT_DOMINANCE: f64 = 1.5;

// Active reasoning gate
pub const ACTIVE_REASONING_MIN_LIVE_NODES: usize = 10;
pub const ACTIVE_REASONING_FAST_PATH_SILENT_TICKS: u32 = 3; // FastPath 连沉默 3 tick 才启慢推理

// Mark decay
pub const MARK_DECAY_US: u64 = 30_000_000; // Q-C 选项 (a)
```

所有常量后续可调，不是宪法。

---

**文档结束 v2。等你 review + 回答 Q-A 到 Q-G。**
