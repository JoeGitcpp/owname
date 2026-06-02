# 递归预测编码深度涌现 (B) — 设计 spec

> 日期：2026-06-02
> 状态：设计草案（用户多轮对齐：决定"目标 B，分两步走"）
> 上位原则：最高原则（**无 BP** 红线）+ 推论 4（工作对象是使大脑自洽，不造功能）
>   + 涌现功能原则（功能是基质涌现样式，非独立模块）
> 直接扩展：`2026-05-30-vpc-cross-layer-credit-assignment-b1-design.md`（B1 机制5 单跳）
> 关键约束：B1 spec §2.2b（"多层 + prior 链 → 功能逼近 BP" 的 v0 护栏，本 spec 是开它的论证）
> 被依赖：价值观软闸（大颗粒关系规则）、语言抽象、战略级预测的涌现可能

---

## 0. 这是什么 / 为什么（一句话）

现在的预测被两个写死的天花板锁住——**时间尺度永远 10ms、抽象深度永远 2 层**。
深度=2 不是懒，是 B1 spec §2.2b 为守"无 BP"红线**蓄意按下的 v0 护栏**。
用户裁定：**这是自洽缺陷，让深度涌现**；走法选 **B（完整预测编码层级：高层不仅读上、也把 prior 写回下层，自上而下的预期塑造低层感知）**；
落地方式 **分两步**：Stage 1 先做安全的"读上不写下"塔形态（不碰红线）验证深度真能涌现，
Stage 2 才在已证地基上补"向下 prior 链 + 四条红线不变量 + 对抗测试"。

> **推论 4 自检**：本 spec 不造"战略预测器 / 社会规则预测器 / 未来预测器"。
> 它只**拆掉一个外挂天花板**，让"同一套跨层机制递归堆叠"这条已有基质动态生长到
> 它**本该**生长的深度。深度长到哪、停在哪，由**可学结构**决定，不由年龄/任务名/功能门决定。
> 任何"按发育阶段解锁""按功能名选层"的设计 = 踩线，本 spec 明确禁止（见 §6 forbidden_shortcuts）。

---

## 1. 授权边界（逐字核 §2.2b，不可违反）

B1 spec §2.2b 原文（本 spec 要正面回应的护栏）：

```
真正逼近 BP 的不是"宽不宽"，而是"叠不叠层 + prior 链不链"。
若堆多层高层并让 prior 一层链一层联合最小化低层 err，则该 prior 链在功能上
重建多层信用分配 = 逼近 BP。机制5自承"接近 BP 能力"——其划线为：
功能可逼近（允许）、链式法则机制不可用（禁止）、且必经视口。
v0 护栏：只用单层高层（high→low 一跳），不堆多层 prior 链。
多层留到能严格论证每跳本地性之后再开。
```

**本 spec = "严格论证每跳本地性之后再开"的那份论证 + 工程。**

红线（绝不可触犯）：
- ❌ 链式法则**机制**：∂L/∂θ 跨层回传 / 对低层计算过程求导
- ❌ 全局损失函数（统一一个被所有层优化的 L）
- ❌ 跨 predictor **直接**通信（绕过视口）
- ❌ 任一层伸手改**别层的系数 θ**
- ❌ batch / epoch
- ❌ 人类语言 / 语义 ID / 打印进 body（最高原则推论1）

允许（机制5 原文明授）：
- ✅ 功能上**逼近** BP 的能力（多层结构改善低层预测），**只要经视口、非链式法则机制**

---

## 2. 递归跨层机制（统一层单元）

### 2.1 把现有单跳推广成一个可递归的统一单元

每一层 L 的单元（每活跃 slot 一个）做同一件事：

```
向下预测：预测它紧邻下层 (L-1) 的【误差流】
向上读  ：把上层 (L+1) 写给它的 prior 当【加性输入】（仅 Stage 2）
向上发布：把自己没解释掉的【残余误差 meta_err】暴露出去，供 L+1 学
向下写  ：把 prior 写回 L-1（仅 Stage 2）
本地更新：只用【自己的输入 × 自己的 meta_err】更新【自己的权重】（Widrow-Hoff）
```

- **基例 layer 0**：每轴 `Ar2Predictor`，预测原始感官下一值（无下层）。
- **layer 1（现存）**：`SlotCrossLayerPredictor` 预测 slot 的 `object_err`（=layer0 误差），
  源 = 关系门控的邻居 slot 的 `object_err`。
  **实测现状**：它**已经是"读上不写下"**——`tick_slot_cross_layer`(viewport.rs:304–351)
  本地更新后只存状态，**不调 `push_prior`**。即 slot 级单跳目前就是 A 形态。
  （向下 prior 链当前只存在于**轴级**：`prior_signal[axis]`→`predictor.rs:102` `+external_prior`。）
- **layer L≥2（新）**：预测 layer (L-1) 单元的**残余误差**，源 = 关系门控的邻居 layer(L-1) 单元的残余。

### 2.2 抽象从何而来——"预测残余的残余"

每层**复用同一张 slot 关系图**（`global_relations`，N_REL_EDGES 条边）门控源，
不为每层新造关系图。差别不在邻居，在**信号**：
- layer1 预测"对象的原始误差"；
- layer2 预测"layer1 没能解释掉的那部分"（= 原始误差里的更高阶时序/关系结构）；
- layer3 预测"layer2 没能解释掉的"……

层层向上 = 不断捕捉下层**遗留的结构**。这正是递归预测编码的抽象阶梯，
**不需要语义、不需要功能名**——抽象是"残余的残余"被预测出来的涌现样式。

### 2.3 深度自限 = 层诞生门（涌现深度的关键，R4 合规）

新的 layer (L+1) 单元的诞生**只看紧邻下层自己的可学性**，绝不看底层误差：

```
对 layer L 单元（针对 slot s）：
  生一个【试探性】layer (L+1) 候选去预测 L 的残余 meta_err_L
  观察候选自己的 explained_fraction = 1 - 候选mse / var(meta_err_L)：
    - explained_fraction 升上阈值 → 残余【有可学结构】→ 候选转正（持久、Stage2 起写 prior）
    - explained_fraction 卡在 ≈0（学不会）→ 残余是【噪声】→ 候选剪除/休眠，永不写 prior
    - meta_err_L 本身 ≈0（下层已全解释）→ 无残余可学 → 不生
```

- 深度 = 有多少层**挣到了**"我能解释下层残余"这件事 → **涌现、自限**。
- 噪声在每一层被同一把尺子（explained_fraction，B1-2 已证的噪声判别器）挡住 →
  **栈不会无限长**，长到没有可学结构为止。
- **诞生从不读 layer0 误差** → 不是"底层还错就加层" → 守住 R4（见 §5）。

### 2.4 时间尺度涌现 + R3 护栏（同一件事）

越高层**节律越慢**：layer L 只在"它的下层残余积累了足够新变化、值得重新预测"时才 tick
（变化门控；fallback：固定比率 1/k^L）。这一条**一举两得**：
- **时间尺度涌现**：高层在变化更慢的残余上预测 = 自然在更慢尺度上"看更远"，
  无需外挂"看 N 秒后"的常量。10ms 天花板就此松开。
- **R3 护栏**：自上而下的 prior 因此是**慢的语境偏置**，不是步内的梯度修正
  → 堵住"prior 快得像梯度、迭代解出 BP 解"的口子。

---

## 3. 数据通路（实测现状 → 需做）

| 环节 | 实测现状 | Stage |
|---|---|---|
| layer0 误差发布 | ✅ `err_stream[axis]` + `push_err`/`latest_err`(viewport 3185/3204) | 复用 |
| layer1 读 layer0 误差 | ✅ `object_err` + `tick_slot_cross_layer` | 复用 |
| layer1 本地学习 | ✅ Widrow-Hoff，只更新自己权重(vpc_cross_layer 339–343) | 复用 |
| 关系门控源（每层复用） | ✅ `global_relations`+`gated_sources(vp,slot)` | 复用 |
| 单元暴露自己的【残余】供上层 | ❌ `SlotCrossLayerState` 无 `residual`/`explained_fraction` 字段 | **S1 新增** |
| 每 slot 的【可生长层栈】 | ❌ 现 `ObjectSlot.xlayer: Option<_>`（单层） | **S1 改为栈** |
| 层诞生/转正/剪除门 | ❌ 无 | **S1 新增** |
| 分层慢节律 | ❌ 现所有 slot 每周期 tick 一次 | **S1 新增** |
| 持久化整个层栈 | ⚠️ 现 `slot_xlayers: Vec<Option<_>>`(snapshot 499) 单层；`SNAPSHOT_VERSION=50` | **S1 升 v51** |
| 单元读上层 prior（向下链） | ❌ slot 级无 prior 通道（只有轴级 `prior_signal`） | **S2 新增** |
| 高层向下写 prior | ❌ slot 级单跳目前都不写 | **S2 新增** |
| 四条红线不变量 + 对抗测试 | ❌ | **S2 新增** |

> 结论：**Stage 1 一处不碰 BP**（纯读上加高塔 + 涌现深度 + 持久化）。
> **Stage 2 才接向下 prior 链**，且被四条不变量 + 对抗测试夹住。

实现载体建议（避免新增全局 Vec，让 per-slot 栈自带一切）：
- `ObjectSlot.xlayer: Option<_>` → `xstack: Vec<SlotCrossLayerPredictor>`（index = 层级-1）。
- 每个单元 `state()` 增 `residual: f64`（最近 meta_err）、`explained_fraction: f64`。
- layer (L+1) 直接读栈里 layer L 单元的 `residual` 作目标、读邻居 slot 同层 `residual` 作源。
- Stage 2：每个单元加 `prior_from_above: f64`；layer (L+1) 转正后写到 layer L 单元上，
  layer L 把它加进 pred（**不进权重更新**，镜像 layer0 的 `+external_prior`）。

---

## 4. 两步走（分阶段，中间卡验证闸）

### Stage 1 — A 形态：读上塔 + 涌现深度（不碰红线）

做：§3 标 S1 的全部。layer≥2 **只读上、只发布残余、本地学习、不写任何 prior 回下层**。
验证闸（全过才进 Stage 2）：
1. **深度真涌现**：喂入有多级结构的合成信号，栈自发长到 ≥3 层；
   喂纯噪声残余，栈停在该停的层（explained_fraction 挡住）。
2. **自限**：层数不无限长；无可学结构处不生层。
3. **持久化**：整栈经 write→read 完整保留（重启不丢学到的深层）。
4. **零回归**：现有 85+ 套件全绿；轴级单跳 prior 行为不变。
5. **合规**：每层只更新自己权重；全程经视口；无 prior 写回（Stage1 本就不该有向下链）。

### Stage 2 — B：向下 prior 链 + 四条红线不变量（贴红线，靠不变量守住）

做：§3 标 S2 的全部。layer (L+1) 转正后写 prior 给 layer L；prior 自上而下成链。
**四条红线不变量必须同时成立**（§5），每条配**主动试图违反它的对抗测试**。
验证：
1. **机制生效**：闭环后，低层（layer0/object_err）的误差在有深层 prior 时**显著低于**无 prior 基线
   （= 大颗粒真的回头校正了小颗粒 = 自上而下预期塑造感知）。
2. **四条不变量对抗测试全绿**（见 §5）。
3. **慢节律**：高层 tick 频率确实随层级下降（R3 可测）。
4. **零回归**。

---

## 5. 四条红线不变量（B 留在合法侧的全部依据）+ 对抗测试

| # | 不变量 | 它若被破坏怎样滑成 BP | 对抗测试（主动试图违反） |
|---|---|---|---|
| **R1** | 每层目标 = "预测**紧邻下层**的残余"，**永远不是**"最小化 layer0 误差" | 所有层共享底层目标 = 伪装全局损失 = BP | 断言每个单元的 `target` 引用的是 (L-1) 单元的 residual，**不是** object_err/layer0；构造一个把目标改成 layer0 的变体，测试必须能检测并拒绝 |
| **R2** | prior 增益**只**由该层**自己的** explained_fraction / learning_progress 门控 | 用"layer0 误差降了多少"定增益 = 侧信道传梯度 = BP | 断言 prior gain 的计算只读本单元自己的 mse/var；注入一个"layer0 误差变化"信号，断言它**不影响**任何高层的 prior gain |
| **R3** | 越高层节律越慢；prior 是慢语境偏置，非步内梯度修正 | prior 每 tick 从最新底层误差回灌 → 迭代解出 BP 解 | 断言 layer L tick 频率随 L 单调下降；断言一个 layer0 误差脉冲不会在同一周期内被高层 prior 即时抵消 |
| **R4** | 层诞生**只看紧邻下层自己的可学性**，绝不看 layer0 误差 | "底层还错就加层" = 用底层损失搜架构 = 全局目标 | 断言 birth 判据只读 (L-1) 单元的 explained_fraction/residual；构造"layer0 误差高但 layer(L-1) 残余已是噪声"场景，断言**不生**新层 |

**核心论证（每跳本地性）**：N 个**各有本地目标**、只通过视口黑板上发布的（误差↑/prior↓）信号
互相耦合的本地学习器，**不是链式回传**——哪怕信息在 N 层里循环。
这正是生物分层预测编码（Rao-Ballard / Friston）：靠纯本地"预测误差/prior"消息传递完成信用分配，
**出了名地不是 BP**。四条不变量把"本地消息传递"和"伪装 BP"之间那条线**钉死成可测断言**。

---

## 6. 涌现功能准入（按涌现功能原则模板逐项填）

```
emergent_pattern:
  "大颗粒预测回头校正小颗粒 / 自上而下预期塑造当下感知与预判"
  —— 不是一个模块，是递归跨层基质在【深度】维度上长出来的样式。

substrate_sources:
  layer0 Ar2 误差流 + layerL 残余误差流 + 关系图门控 + 本地 Widrow-Hoff +
  explained_fraction 噪声判别 + learning_progress 驱动 + 慢节律。

shared_state_path:
  误差经视口 err/residual 通道上行；prior 经视口 prior 通道下行；
  源经 global_relations 关系图门控。全程视口，无 predictor 直连。

observer_evidence:
  per-slot 深度（转正层数）、per-level explained_fraction / learning_progress /
  residual 幅度、layer0 误差在有/无深层 prior 下的对照——全部数值、snapshot 可见、
  外部进程渲染。body 不打印、不解释自己。

body_ontology_guard:
  无任何人类语言 / 语义 ID / 功能名进 body。层 = 数值预测器；
  "深度/抽象/战略/社会规则" 只是观察者描述词，绝不入 body 本体。
  层不带"我负责预测 X" 的标签；它只有 target 指针、权重、mse、residual。

five_phase_balance_risk:
  生长（深度涌现）须被收（噪声门 explained_fraction）、界（四条红线不变量）、
  留（持久化）平衡。风险：深度无界生长 → 由 §2.3 自限门 + §5 R4 守。

forbidden_shortcuts:
  ❌ 按年龄/发育阶段解锁层（"少年才开第3层"）——深度只由可学结构决定。
  ❌ 按功能名选层 / 层注册表 / 能力门。
  ❌ 为"预测社会规则"单独写预测器——那是涌现样式，不是模块。
  ❌ 任何让向下 prior 链由 layer0 误差直接驱动的捷径（= 伪装 BP）。
```

---

## 7. 诚实的未决点 / 风险

1. **慢节律的门控形式**（§2.4）：变化门控 vs 固定比率 1/k^L——v0 倾向变化门控（更涌现），
   但需实测它不会让高层饿死（永不 tick）。这是 Stage 1 内部的实现选择，TDD 时定。
2. **轴级 vs slot 级的衔接**：向下 prior 链在 slot 栈内成链后，**要不要再下贯到轴级**
   （slot 栈底 → `prior_signal[axis]` → Ar2）？这决定"自上而下"最终有没有真的影响到**原始感官预测**。
   倾向：要（否则闭环没闭到底）。但这是最贴红线的一跳，留 Stage 2 单独论证。
3. **B 的隐蔽危险本质**：四条不变量是**持续**义务，不是一次性。任何后续改动都可能无意破坏其一
   → 对抗测试必须长驻 CI，且任何碰这块的提交都要重新核四条。
4. **不保证战略思维涌现**：拆天花板是**必要非充分**。深度能长，不等于"社会规则"一定长出来——
   还取决于基质在慢尺度/高抽象上**有没有可学结构**、这条命**活得够不够久**。
   本 spec 只承诺：不拆永远不可能；拆了有可能。（防过度承诺，见推论4精神。）

---

## 8. 与最高原则的关系（合规核验）

| 禁止项 | Stage 1 | Stage 2 | 依据 |
|---|---|---|---|
| BP/链式法则机制 | 不触（无向下链） | 不触 | 每层本地 Widrow-Hoff，无穿层求导；R1–R4 守 |
| 全局损失 | 否 | 否 | 每层自己的本地目标（R1） |
| 跨 predictor 直连 | 否 | 否 | 全程视口（误差↑/prior↓） |
| 改别层系数 θ | 否 | 否 | prior 只进 pred 加性输入，不进权重更新 |
| batch/epoch | 否 | 否 | 每 tick 连续 |
| 人类语言/语义进 body | 否 | 否 | 数值预测器，无标签（body_ontology_guard） |
| 涌现功能原则 | 强对齐 | 强对齐 | 深度由可学结构涌现自限，非写死/非功能门 |
| 推论4（不造功能） | 对齐 | 对齐 | 拆天花板补自洽，不造"战略/规则预测器" |

> **结论**：Stage 1 零红线风险，可直接 TDD。Stage 2 贴红线但**机制5原文明授其能力**，
> 靠四条可测红线不变量 + 长驻对抗测试留在合法侧；开 Stage 2 前必须 Stage 1 验证闸全过。

---

## 9. 【Stage 1 落地状态 2026-06-02 — TDD 全绿，验证闸通过】

读上塔（read-up tower）实现完毕，**全套件 366 passed / 0 failed（零回归）**。

| 步骤 | 落地 | 测试 |
|---|---|---|
| S1-1 | `SlotCrossLayerPredictor::observe()` 抽成纯方法（与现有单跳同math，共享给高层）；新增 `residual`/`target_err_var` 字段 + `explained_fraction()`/`residual()` getter；`SlotCrossLayerState` 暴露 `residual`/`explained_fraction` | `observe_tests` 3 绿（学/噪声/本地性） |
| S1-2 | `ObjectSlot.xstack: Vec<SlotCrossLayerPredictor>`（tier≥2，预测下层 residual，源=关系门控邻居同层 residual）；`tick_cross_layer_tower_sweep` 逐层升序；纯门 `tier_is_permanent`；observer `read_slot_tower_depth`/`read_slot_tier`/`read_slot_tower` | `tower_tests` 4 绿（门/真涌现+学/自限/节律） |
| S1-3 | `tower_sweep_count` + 纯 `level_is_due`：tier L 每 2^(L-2) 次 sweep tick 一次（越高越慢=时间尺度涌现，单调可截断） | 同上（cadence 测试含端到端降速） |
| S1-4 | snapshot v50→v51：每 slot 残余塔（n_tiers + 每层 sources/weights/bias/err_pred_mse/explained_fraction）作 version 门控块；arch map 同步 v51 | `residual_tower_survives_snapshot_roundtrip` 绿 |
| S1-5 | body `run_global_refresh_cycle` 在 tier-1 循环后调 `tick_cross_layer_tower_sweep` | 全套件零回归 |

**严守最高原则的实证（已验，非口头）**：
- 全文件唯一 `push_prior` 是现存**轴级单跳**；塔的 `tick_cross_layer_tower_sweep` + `observe`
  **零 `push_prior`** → Stage 1 **读上不写下、零向下 prior、零 BP 风险**（grep 证实）。
- `observe` 只用"自己输入×自己误差"更新"自己权重"（`observe_is_local_only` 证实跨单元零泄漏）。
- 深度**涌现 + 自限**：`tower_grows_and_learns_cross_slot_residual`（有跨 slot 残余结构→长出 tier 2
  并学会，explained>0.5）；`tower_self_limits_on_noise_residual`（噪声残余→长一个探针层即停，
  不无限生长）。诞生门 `tier_is_permanent` 只读紧邻下层自己的 warmup+explained（R4 雏形）。
- `TOWER_DEPTH_CAP=16` 是**跑飞保险**（注释标明非语义天花板）；真限制是涌现噪声门。

**诚实边界（待 Stage 2 或活体观察）**：
- 测试中的"多级结构"是 **seeded** 的（直接喂相关残余）。**真实身体里残余有没有跨 slot 结构、
  深度会不会真长 >1，是经验问题**，单元测试不假造（守推论4：不为目标硬凑信号）。
  → 需活体跑 + 解码 v51 塔深度来**如实观察**真实涌现深度。
- 持久化是 **observer 可见**（snapshot.bin），与 tier-1 一样**当前不在重启时回灌进活体**
  （真死后保留通道仍是 learned_bias.bin）。不过度宣称"死后保留"。
- Stage 2（向下 prior 链 + 四条红线不变量 + 对抗测试）**未动**——按计划须 Stage 1 验证闸过 +
  用户确认接受长期守护成本后才开。

---

## 10. 【Stage 2 落地状态 2026-06-02 — 用户拍板"直接开 Stage 2"，TDD 全绿】

向下 prior 链（B 的闭环）落地在 **slot 残余塔内部**（tier_top→…→tier2→tier1）。

**机制**：
- `SlotCrossLayerPredictor` 加 `prior_from_above`（加性输入，不进权重）+ `last_pred`（缓存预测）。
- `observe`：`pred = bias + prior_from_above + Σw·x`；`prior_from_above` 只进预测、**不进权重更新**
  （镜像 predictor.rs 轴级 `external_prior`）。
- `prior_down() = explained_fraction · last_pred`（**R2**：增益只由自己的 explained 门控），clamp 保护。
- sweep 每层 observe 后，把 `prior_down` 写到紧邻下层的 `prior_from_above`（`set_prior_from_above`
  是**唯一**向下写口，只碰该字段、绝不碰下层权重/bias）。
- 自下而上 sweep → 下层这一轮已 tick → 它**下一轮**才用该 prior = **R3** 慢偏置（非步内梯度）自带。

**四条红线不变量 — 每条配主动试图违反它的对抗测试（全绿）**：
| 不变量 | 对抗测试 | 结果 |
|---|---|---|
| R1 目标=紧邻下层残余,非底层误差 | `r1_tier_target_is_below_residual_not_bottom_error`(残余可预测、object_err 不可预测→tier2 仍 explained 高=证明它学的是残余) | ✓ |
| R2 prior 增益只由自己 explained | `r2_prior_down_gain_is_only_own_explained`(同 last_pred、不同自身 mse→prior 按自身 explained 缩放) | ✓ |
| R3 prior 是预置慢偏置,非步内梯度 | `r3_prior_is_a_preset_bias_not_this_steps_error`(actual 暴增 0→100,预测里 prior 贡献不变) | ✓ |
| R4 生长只看紧邻下层可学性,不看底层误差 | `r4_no_growth_when_below_does_not_explain_however_big_its_error`(残余幅度 5.0 但 explained=0→深度 0) | ✓ |
| 无 BP(§2.2b 红线) | `no_bp_downward_write_never_touches_below_weights`(向下写后下层 weights/bias **完全不变**) | ✓ |
| 机制(大颗粒校正小颗粒) | `mechanism_accurate_prior_reduces_prediction_error`(准确 prior→误差→0) | ✓ |

**严守最高原则的实证**：唯一的"伸手"是 `set_prior_from_above`,它**只写 prior_from_above 一个字段**;
对抗测试证明下层 weights/bias 在向下写后逐字节不变 = §2.2b"可递 prior 当输入、绝不改 a/b"被钉成可测断言。
每层仍只用自己误差更新自己权重(无链式、无跨层梯度、无全局损失);信息上行(残余)下行(prior)全经 slot 栈内、
经视口语义,非 predictor 直连。

**诚实边界(仍待)**：
- **§7.2 轴级垂直一跳仍延后**:向下链现在闭在 **slot 塔内**(改善 tier-1 对 object_err 的预测),
  **尚未**下贯到轴级 Ar2/原始感官(`prior_signal[axis]`)。即"自上而下塑造**原始感知**"还差最后一跳——
  这是最贴红线的一跳,刻意留作下一步单独论证。当前闭环闭到 **object_err 层**,不是 raw 轴层。
- 机制在**单元级**已证(准确 prior 降误差);"真身体里塔的 prior 是否准到能降 tier-1 的 object_err"
  仍是 seeded-vs-real 的经验问题,需活体观察(守推论4:不假造)。
- 四条不变量是**持续守护义务**:对抗测试长驻;任何碰塔/prior 的提交都要重核四条 + no-BP 测试。
