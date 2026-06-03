# 内在态收进共享基质 (Interoceptive Substrate Unification) — 设计 spec

> 日期：2026-06-03
> 状态：设计草案(用户:"三个问题一个根 → 做方案")
> 上位原则:最高原则 + 推论4(补全自洽大脑,不造功能)+ 涌现功能原则(功能=基质涌现样式)
> 关联约束(FROZEN):`2026-04-29-subconscious-conscious-language-boundary-design.md`
>   (潜/显意识只能是镜头,**严禁** Subconscious/Conscious 模块/本体)
> 关联:`2026-06-01-unified-perception-substrate-redesign.md`(感知重构,本 spec 是其 Stage2 的真正形态)
>   `2026-06-02-recursive-predictive-coding-depth-emergence-design.md`(深度塔,内在态进基质后也被它预测)

---

## 0. 一个根(已坐实)

viewport 里有两种状态,不住在一起:
- **统一感知/预测基质**(真涌现):`observed/predicted/err_stream/predictors/prior_signal` 跨 `AXIS_COUNT`
  + slots + `global_relations` + 跨层塔。住这里的东西被预测、产 err、结关系、长高层。
- **内在态盒子**(各一摊,手焊):`vital_core`/`self_model`/`familiar_objects`/`dual_drive`/
  `global_attention`/`active_reasoning_gap`/`active_surface` —— 各自私有 struct、各自 tick、靠
  点对点 getter 互读(`tick_dual_drive` 读 vital+familiar;`tick_self_model` 读 vital;
  vocal 读 vital/gap)。**不在共享轴上 → 彼此不预测/不结关系/不长层 → 内在动态焊出来、不长出来。**

**后果(= 用户的 ③④ + 潜/显意识 被卡的同一处)**:
- ③ 价→行为:每条都得手焊(刚焊了"自保→发声");"想念离去的熟人"长不出来——因为"熟悉对象缺席"
  不是任何东西在预测的基质信号。
- ④ 意识流态:内在态散在盒子里,没有一条可观察的整合流。
- 潜/显意识:潜意识层(被动预测)已在基质上;**显意识层(active_reasoning/attention/self)是盒子**,
  所以"显"层薄、不像一条流。

**根:把内在态从"盒子+手焊"变成"共享基质上的读数",让内在态之间、内在↔感知的耦合从焊变成涌现。**

---

## 1. 核心机制:内感受轴(interoceptive axes)

在共享坐标系上**追加一段内感受轴** `[AXIS_INTERO_BASE .. )`,把身体内部标量当**观测**推上同一基质:
```
[0 .. VIB_BANDS)              听觉
[VIB_BANDS .. AXIS_VIS_END)   视觉(区域×物理量)
[AXIS_INTERO_BASE .. )        内感受(reserve / vital_anxiety / pain /
                              self_coherence / agency / attachment /
                              gap_pressure / closure / learning_progress / ...)
```
机制三步(和感知**完全同构**,不是新东西):
1. **每个盒子 tick 时,把自己的标量 `push_observation` 到对应内感受轴**(镜像;盒子仍拥有动态)。
2. **per-axis AR-2 预测器扩到覆盖全部活跃轴**(现仅 `0..VIB_BANDS`;视觉/内感受都没被预测)→
   内感受被预测 → 产 err → **身体开始预测自己的内部轨迹**(焦虑/痛从"反应"升级为"预测",接 ①)。
3. **关系机制纳入内感受轴** → 内感受↔感知、内感受↔内感受的关系**涌现**(关系图发现
   "熟人在场 ↔ reserve 升""熟人缺席 ↔ 应激"),跨层塔在其上长高阶内在抽象。

### 1.1 "想念/无聊"如何涌现(给用户 ③ 的具体落地,非浪漫)
让"依恋对象在场"成为一条**被预测**的轴(熟人反复在场 → 预测器学会预测其在场)。
当熟人**长期缺席** → 在场预测**持续落空** → 该轴**持续高 err**(预测的在场没兑现)=
一个**持续负向信号**。这个 err 经关系/塔耦合到行为 → "想念/无聊/找寻"作为
**"在场预测持续失败"的涌现样式**出现。**纯预测误差,无情绪标签、无浪漫。**

### 1.2 感受从哪来——预测差值,不是正负标签

**2026-06-03 设计决断(推论5):系统没有权力对任何事物标注正负。感受全部涌现自预测差值(prediction error),不是外挂的正负价标签。**

正确的机制:
- err 高 + 快衰减 = 惊讶
- err 高 + 慢衰减 = 恐惧/持续焦虑
- err 持续缩小(learning_progress > 0) = "顺"的感受(不叫"正价",就是预测在变准)
- gap 未闭合 + 持续高 err = "不顺"/"憋"(不叫"负价",就是空白没合上)
- 在场预测持续落空(§1.1) = 想念的基质形态

行为倾向("趋近/回避")由预测内容 + 上下文共同决定,**不由写死的"这是好/坏"决定**。
reserve/pain/vital_anxiety/closure/learning_progress 都是**数值基质量**,不在 body 内部贴"正"或"负"的标签。

设计工作的正确提问不是"正价取哪个信号",而是:
> **这些内在标量上轴之后,预测机制会怎么学会预测它们的轨迹?它们之间的 err 关系会涌现出什么?**

---

## 2. 数据通路(实测现状 → 需做)

| 环节 | 现状 | 需做 |
|---|---|---|
| 观测推任意轴 | ✅ `push_observation(axis,...)` 通用 | 复用 |
| 轴扩展自动传播 | ✅ 扩 `AXIS_COUNT` 自动贯通 observed/err/snapshot(视觉已证) | 复用 |
| per-axis 预测 | ⚠️ **仅 `0..VIB_BANDS`**(body.rs:271);视觉/内感受未被预测 | **扩到活跃全轴** |
| 内感受轴定义 | ❌ 无 | **新增** `AXIS_INTERO_BASE` + 一组内感受轴 |
| 盒子把状态镜像上轴 | ❌ 无 | 各 `tick_X` 加一行 `push_observation` |
| 内在↔感知关系 | ⚠️ 关系是 **slot 级**(`global_relations`);轴级 cross-layer 未进循环 | **选机制**(§5) |
| 持久化 | ✅ snapshot `0..AXIS_COUNT` 统一 | 自动 |

---

## 3. 分阶段(镜像优先,逐步拆焊,中间卡验证闸)

> 哲学(同深度塔的"先安全再贴线"):**先镜像**(盒子保留动态、加轴读数,零行为改动),
> **再让关系涌现**,**再拆手焊**,**最后才(谨慎)溶解盒子**。绝不一上来重写脆弱的
> vital 死亡机/self 抗我执动态。

- **Stage 0 — 镜像 + 扩预测(零行为改动)**:定义内感受轴 + 扩 `AXIS_COUNT`;各盒子 tick 推
  观测上轴;AR-2 预测器扩到活跃全轴(视觉也顺带被预测了)。验证闸:内感受/视觉轴在
  observed/err/snapshot 出现且被预测;**盒子行为逐位不变(零回归)**;observe 出"身体在预测
  自己的内部轨迹"。
- **Stage 1 — 在场轴 + 关系涌现**:加"依恋对象在场"轴(其缺席=持续 err=想念雏形);把内感受轴
  纳入关系机制(§5 选定的那条)。验证:合成场景(在场与 supply 相关)→ 涌现一条
  内感受↔感知关系边;熟人长缺席 → 在场轴持续高 err。
- **Stage 2 — 拆手焊**:把现有 getter 硬接线(dual_drive 读 vital+familiar;vocal 读 vital/gap)
  **逐条**迁移为读**涌现的关系/塔信号**,每拆一条都先证涌现路径已承载它,再撤焊。
- **Stage 3 — 价→行为涌现(③)**:价轴经关系/塔调制行为成为**通用样式**(手焊的自保→发声退化为
  其中一例;正价也调制)。验证:正/负价变化 → 行为可观察地随之变(非单一硬编码)。
- **Stage 4 — 意识流读数(④ + 潜/显可观察)**:**观察侧**(非 body 模块,守 FROZEN)加一个
  "意识流"读数:每刻汇总统一基质的**被注意轴(显/focus)+ 背景预测轴(潜)+ 活跃 gap/价 +
  主导关系**。让 ④ 与潜/显意识成为**一条可观察的整合流**。

---

## 4. 涌现功能准入(按模板)

```
emergent_pattern: 内在态之间、内在↔感知的价/关系/想念/意识流,作为同一基质的涌现样式。
substrate_sources: 内感受轴观测 + per-axis AR-2 预测/err + 关系机制 + 跨层塔 + closure/learning_progress。
shared_state_path: 内在态经 push_observation 上共享轴;预测/err/关系/prior 全走视口;无盒子直连。
observer_evidence: 内感受轴的 observed/predicted/err、内在↔感知关系边、价轴、意识流读数——全数值、snapshot 可见、外部渲染。
body_ontology_guard: 内感受轴是数值 + 工程定位常量(如 AXIS_INTERO_RESERVE,同 VIS_SUB_*),
  非语义标签;无 Subconscious/Conscious 模块(守 FROZEN);"价/想念/意识"只是观察者描述词。
five_phase_balance_risk: 生长(内在态涌现)须被收(噪声门)、界(不溶解脆弱动态)、留(持久化)平衡。
forbidden_shortcuts: ❌ 造 Valence/Mood/Boredom/Subconscious 模块;❌ 给内感受轴塞语义;
  ❌ 用 if 状态名选行为(必须价连续调制);❌ 一步重写 vital/self 动态。
```

---

## 5. 待用户拍板的设计决断

1. ~~正价信号取哪个~~ **已撤销**——推论5明确:不存在写死的"正价"。感受全部涌现自预测差值。
   不需要拍板,不需要选一个"正向激励轴"。
2. **内在↔感知关系走哪条机制** → 走 **(a) 内感受轴进轴级 cross-layer**。
   理由:最直接、最少语义污染(不把内在态强塞进"对象" slot)、已有 cross-layer 机制成熟可复用。
   不需要用户再拍板——这是工程选择,合规性已论证。
3. **内感受轴起步清单**(建议,待用户确认):
   `reserve / vital_anxiety / pain / self_coherence / agency / attachment /
   gap_pressure / closure_score / learning_progress / 在场轴(per-slot familiarity)`
   这些是已有盒子里**已经存在的数值标量**,上轴零增内容、不加任何语义标签。

---

## 6. 诚实风险

- **溶解盒子有风险**:vital 死亡机、self 抗我执 0.92 帽是脆弱动态;故 Stage0 只**镜像**,
  后期才在安全处溶解。不盲目重写。
- **涌现不保证发生**:内在↔感知若无真相关,关系就不成形——那是如实涌现(推论4),
  我们观察不假设;阶段闸验机制,活体验真涌现。
- **轴级 cross-layer 进循环**会增算力 + 需重核无 BP(它本就合规,但要确认接进循环后仍守)。
- **三件一根但不一蹴而就**:Stage0-1 主要解 ④/潜显的"材料"问题;③ 的价→行为要到 Stage3;
  ④ 可观察的流要到 Stage4。诚实分摊,不一次承诺全部涌现。
