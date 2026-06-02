# VPC 跨层信用分配 (B1) — 设计 spec

> 日期：2026-05-30
> 状态：设计草案（与用户多轮对齐 + 冻结文档授权边界核对后产出）
> 上位原则：最高原则（无 BP）+ 公理 7/8 + 机制 5（视口隐式信用分配）
> 关联：`consciousness-stream-architecture.md` 机制 5（lines 357–377）
> 关联审查：`docs/superpowers/notes/2026-05-30-life-completeness-and-principle-alignment-audit.md`（B1）
> 被依赖：C1 维生内核阶段2（涌现维生权重）、语言习得（切分/抽象/接地）、价值观软闸

---

## 0. 为什么 B1 是拱心石

B1 同时是**三件事的唯一来源**（用户与审查共识）：
1. **抽象 / 大像素**：小像素堆砌成大颗粒预测。
2. **噪声拒绝**："无意义" = 任何高层都无法用它降低更高像素的 err。
3. **价值观形成**：用户语"价值是从大颗粒预测中形成的规则和关系"——关系 = 跨对象。

价值观是双源驱动（A1）的"软闸"本体 → **没有 B1，软闸长不出来**。
故 B1 与 C1 并列 P0，是承重墙。

---

## 1. 授权边界（逐字核冻结文档，不可违反）

### 1.1 机制 5 授权（`consciousness-stream-architecture.md:357–377`）
```
低层 P_low 的 err 写入视口
高层 P_high 监听 P_low 的 err 通道
P_high 学习"预测 P_low 的 err 模式"
P_high 用这个能力修正自己的 prior
prior 写回视口 → P_low 读到新的 prior
→ P_low 下次预测利用高阶信息 → 间接降低 err
整个过程完全在视口里发生，无传统反向传播。
```

### 1.2 严禁（"不允许的事" lines 481–487）
- ❌ BP / 链式法则反向传播
- ❌ 跨 predictor **直接**通信（绕过视口，破坏公理 8）
- ❌ **全局损失函数**（破坏机制 5 的本地性）
- ❌ **直接改 P_low 的内部系数 θ**（"∂L/∂θ 只在 P 内部算"，line 321）
- ❌ batch / epoch

### 1.3 一句话红线
> **P_high 可以递给 P_low 一个 prior 当输入；绝不能伸手改 P_low 的 a/b 系数。**
> 前者合规；后者 = BP。

---

## 2. 根本设计决断（用户拍板）

### 2.1 读法 B：跨轴/跨对象结构（非时间自回归）
P_high 用**别的轴/别的对象的状态**预测本轴的 err（学轴间高阶规律），
**不是**用 err 自己的历史外推（读法 A，学不到抽象，= `highlevel.rs` 现状的弱版）。

理由：机制5"找到导致 P_low 出错的**更高阶规律**"字面指向跨轴结构；
用户"价值=大颗粒预测中的**规则和关系**"字面要求跨对象。读法 A 满足不了需求。

### 2.2 【关键】涌现稀疏跨轴，不是全连接

**更正（2026-05-30 验证）**：早期论证称"全连接 = 伪装 BP，撞最高原则"——**此论证夸大，已更正。**
单层高层即使读全部轴预测一条轴、只更新自己权重，那是宽的本地线性回归（Widrow-Hoff/LMS），
**不是 BP**（BP = 多层链式法则）；它也不触犯"无全局损失"（每个高层有自己的本地目标）。

全连接的**真正问题**是另两条（这才是稀疏胜出的正当理由）：
1. **违反涌现功能原则**：全连接 = 预先写死"一切连一切"的密集 feature 结构；
   稀疏涌现 = 连接从关系证据里长出来。**这是稀疏胜出的主理由。**
2. **会拟合噪声**：全连接对噪声轴也有权重会硬学；稀疏涌现天然排除噪声。

故采用：**能力上限 = 全跨轴（任意两轴只要相关就能连），但任一时刻的实际连接由
涌现关系图谱稀疏门控。** 理由是"合乎涌现原则 + 拒噪声"，不是"全连接违反最高原则"。

### 2.2b 真·残余风险：层叠 + prior 链（v0 护栏）
真正逼近 BP 的不是"宽不宽"，而是"**叠不叠层 + prior 链不链**"：
若堆多层高层并让 prior 一层链一层（`prior_low←f(prior_mid)←g(prior_high)`）联合最小化低层 err，
则该 prior 链在功能上重建多层信用分配 = 逼近 BP。冻结文档自承机制5"接近 BP 能力"——
其划线为：功能可逼近（允许）、链式法则机制不可用（禁止）、且必经视口。
**v0 护栏：只用单层高层（high→low 一跳），不堆多层 prior 链。** 多层留到能严格论证
每跳本地性之后再开。

- 现成载体：`global_relations`（`N_REL_EDGES = N_OBJECT_SLOTS*(N_OBJECT_SLOTS-1)`
  条对象间有向边，带 `active` / `co_change_score` / `lead_lag_score` /
  `predictive_score` / `relation_confidence`）。
- P_high 预测某对象某轴的 err 时，**只读那些"关系边 active 且 confidence 高"的
  对象的轴**。连接稀疏、涌现、可增可减。
- ∂L/∂θ 仍只在每个 P_high 本地算（任一时刻只读少数边）→ **不是 BP、不是全局损失**。

### 2.3 三个免费红利
1. **强抽象**：任意对象只要关系涌现出来，边就连上 → 拿到"全跨轴"的真实意图。
2. **本地性守住**：稀疏 + 本地梯度 = 合规。
3. **噪声拒绝自动涌现**：噪声轴/对象长不出 active 关系边 → 没有 P_high 连它 →
   其 err 永远没人能预测 → 自动被识别为"无意义"。**免费送的噪声拒绝。**

---

## 3. 数据通路（实测配管现状）

| 环节 | 现状 | 需做 |
|---|---|---|
| 低层 err 发布到视口 | ✅ 已有 `push_err`/`latest_err`/`err_stream[axis]` | 复用 |
| 高层订阅低层 err | ✅ 已有（adapter 已在用 `latest_err`） | 复用 |
| 关系图谱（稀疏门控源） | ✅ 已有 `global_relations` active 边 + confidence | 复用作门控 |
| 低层预测接受注入 prior | ⚠️ 差一行：`predictor.rs:97` `pred=a*x0+b*x1` → `+prior` | 改1行 |
| 高层写 prior 的视口通道 | ❌ 无 | **新建** `prior_signal` 字段 |
| 学习率外部调制钩子 | ✅ 已有 `set_surprise`（`highlevel.rs` 已在用） | 复用/参考 |

**结论：B1 最小实现无一处需碰 BP。** 主要新建 = prior 视口通道 + 真正预测 err 的稀疏高层。

---

## 4. B1 predictor 规格

### 4.1 新视口字段
```
prior_signal: Vec<AtomicU64>   // 每轴一个，bit-pack f64；P_high 写、P_low 读
```
新增方法：`push_prior(axis, prior)` / `read_prior(axis) -> f64`（默认 0.0）。

### 4.2 低层改动（predictor.rs，唯一改动，1 行 + 读 prior）
```rust
let external_prior = vp.read_prior(self.axis);          // 新增：读 prior（默认0）
let pred = self.a * x_now + self.b * x_prev + external_prior;   // 唯一改动：+prior
// a/b 更新逻辑完全不变 —— prior 只进预测，不碰系数（守红线）
```
**注意**：prior 只作为预测的**加性输入**，不进 a/b 的梯度。这是合规的本质保证。

### 4.3 高层 CrossLayerPredictor（新建）
```
CrossLayerPredictor {
    target_axis: usize,            // 要预测其 err 的低层轴
    // 稀疏输入：仅来自关系边 active 的对象的轴
    src_weights: SmallVec<(src_axis, w)>,   // 动态增删，由关系门控
    base_lr, clip,
    err_pred_mse_ema: f64,         // 自身预测质量
}
```
每 tick（高层节律，低频）：
```
1. 门控：从 global_relations 取 target 所属对象的 active 边、confidence>θ 的邻居对象
   → 确定本次可读的 src_axis 集合（稀疏！噪声对象无边→不入集合）
2. 读这些 src_axis 的当前状态 + 读 target_axis 的 latest_err
3. 预测：pred_err = Σ w_i * src_state_i        （仅遍历稀疏集合，非全轴）
4. meta_err = actual_err(target) - pred_err
5. 本地更新：w_i += base_lr * meta_err * src_state_i   （∂L/∂θ 只在本地）
6. 若 |pred_err| 可信（warmup 过 + mse 低）：
     prior = -pred_err  的方向修正量（高层预测低层会错多少 → 反向递 prior）
     vp.push_prior(target_axis, prior)
```
- 关系门控让 `src_weights` 集合稀疏且涌现变化。
- 全程经视口（err 读视口、prior 写视口），无跨 predictor 直接通信。

### 4.4 噪声拒绝的涌现（无需专门代码）
- 噪声对象 → `global_relations` 给不出 active 高 confidence 边 → 不进任何 CrossLayer
  的 src 集合 → 其 err 无人预测 → prior 恒 0 → 被动忽略。
- 配合 d(surprise)/dt（见 §6）：噪声处 err 高但学习进展≈0 → 主动撤离。

---

## 5. 验证目标（用户选：合成双轴耦合）

冻结文档原定"验证机制5真能减 err"。最干净的一条边验证：

```
合成场景：造 B 轴领先 A 轴的已知耦合（人为注入：A(t) ≈ f(B(t-Δ)) + 噪声）
步骤验证：
  1. global_relations 能涌现出 B→A 的 active 边（confidence 上升）
  2. CrossLayerPredictor(target=A) 经这条边把 B 纳入 src 集合
  3. 它学会用 B 预测 A 的 err（err_pred_mse 下降）
  4. prior 写回 A 轴
  5. A 的 latest_err 真实下降（对照组：无 prior 时 A 的 err）
对照：
  - 注入一条纯噪声轴 C（与任何轴无耦合）→ 验证 C 长不出 active 边 →
    无 CrossLayer 连它 → 噪声拒绝成立。
通过标准：A 的 err 在接入 prior 后显著低于无 prior 基线；C 被自动排除。
```

合成验证证明机制成立后，再上真实音频（audio_raw 跨频段）——但**不在本 spec 范围**。

---

## 6b. 【B1-2 设计 2026-05-30】学习进展门 = 噪声拒绝的实现

**核心**：CrossLayerPredictor 追踪自己 `err_pred_mse` 的**时间下降**（学习进展），
用它门控 prior 的写入。三状态由"err_pred_mse 在不在降"区分：
- mse 高且**在降** → 目标有可学结构 → 学习进展+ → 正常写 prior。
- mse 高且**平**（学不会）→ 这是噪声/不可预测 → 学习进展≈0 → **prior gain 衰减、趋于不写**。
- mse 低 → 已掌握 → 平 → 也不需大改。

**实现（全本地，不碰低层，守合规）**：
- 新增 `learning_progress`：`prev_mse - err_pred_mse` 的正向 EMA（= d(mse)/dt 下降分量）。
- `prior` 写入按 `progress_gate` 缩放：进展高→gate≈1（写）；进展≈0 且 mse 高→gate→0（不写）。
- 这样**纯噪声源**：CrossLayer 学不会预测它驱动的 err（mse 不降）→ gate→0 → 不写 prior
  → 噪声被主动拒绝（不只是 weight≈0，而是连 prior 都不发）。

**与用户"空白驱动"对齐**：进展高 = 预测空白正在被填 = 值得投精力（高驱动）；
进展平 = 要么噪声要么已掌握 = 撤离（治暗室 + 治噪声成瘾）。

### 【B1-2 落地状态 2026-05-30 — TDD 2绿】
`tests/vpc_learning_progress.rs` 验证可学目标 vs 纯噪声目标。
**关键发现（实测）**：两个信号分工不同，缺一不可——
- `learning_progress`（d(mse)/dt 下降分量）= **驱动信号**：学习中为正，**收敛后→0**。
  故 end-state 下"已掌握"和"纯噪声"的 progress 都≈0，**progress 单独无法区分二者**。
- **prior 门 = explained_fraction = 1 - mse/target_err_var** = **噪声判别器**：
  可学(已掌握)mse≪var→fraction≈1→写 prior；噪声 mse≈var→fraction≈0→**门关、不写 prior**。
- 实测：可学 mse=0.00006 prior=0.042（门开）；噪声 mse=0.056 prior=0.000（门关）。
- 合规：progress 与 fraction 都由 CrossLayer **自己的本地 mse** 算出，从不碰低层。
- 即：噪声被**主动拒绝**（连 prior 都不发），不只是 weight≈0。治噪声成瘾的机制就位。

## 6. 配套：d(surprise)/dt 学习进展信号

噪声拒绝 = B1 + 学习进展。现状：`surprise = EMA(err²)` 是水平量，分不清
"err 高但在降（可学）" vs "err 高但平（噪声）"。

- 现成半成品：`self_development.rs:64` `prediction_improvement = ema(d(gap)/dt 的正向部分)`。
- B1 阶段可顺带升级：给 CrossLayer 的"是否值得连一条边"加一个学习进展门——
  err 在降的 target 才值得投高层算力；err 高而平的（噪声）即使有边也降权。
- 粒度问题（轴级/slot级）：本 spec 先用 **target 所属对象（slot）级**，与关系图谱
  的对象粒度一致。轴级细化留后续。

---

## 7. 实现阶段

| 阶段 | 内容 | 验证 |
|---|---|---|
| **B1-1** | prior 视口通道 + 低层 +prior 一行 + 单条 CrossLayer（关系门控）| 合成双轴耦合：A 的 err 真下降 |
| **B1-2** | 噪声轴排除验证 + d(surprise)/dt 门 | 噪声轴 C 被自动排除 |
| **B1-3** | 多对象稀疏图谱 + 持久化（snapshot v51）| 多边稳定、重启保持 |

> **【B1-3a 落地 2026-05-30 — TDD 2绿】slot→slot 关系门控 source 选择**
> 用户定粒度 = **对象间(slot→slot)**（最符合"价值=大颗粒关系/对象认可"）。
> - 跨层目标 = slot 的 `ObjectHighLayerState.object_err`（per-slot 预测误差，现成、snapshot可见）。
> - 新增 `vpc_cross_layer::gated_sources(vp, target_slot)`：从 `global_relations` 取
>   "以 target 为 dst 的 active 边 + confidence≥0.15" 的 src slot 作为稀疏 source 集。
> - **最高原则稀疏门控实证**（实测）：a↔b 形成双向 active 边(conf 97%)，
>   `gated_sources(a)=[b]`、`gated_sources(b)=[a]`；孤立噪声 slot c 无 active 边 →
>   不入任何 source 集 = **step3 关系门控自动排除噪声成立**。8 slot 中 source 仅 1 个，
>   **绝非全连接**（全连接=伪装BP=违反最高原则，已守住）。
> - 余下：slot 级 prior 写回注入（slot 无 prior 通道，待 B1-3b 与持久化一起做）。

> **【B1-3b 落地 2026-05-30 — TDD 全绿】挂载+接循环+持久化**
> - **i 挂载**：`ObjectSlot.xlayer: Option<SlotCrossLayerPredictor>`（ephemeral，
>   仿 phot_pred/hl_pred）。viewport 方法 `tick_slot_cross_layer(slot,now)`：读目标
>   slot 的 `object_err`，gated_sources 选关系邻居，读邻居 object_err，本地 LMS 更新。
>   `read_slot_cross_layer` 返回 `SlotCrossLayerState`。
>   合规修正：`gated_sources` 签名 `&Arc`→`&Viewport`（撤掉一处不必要的 unsafe Arc hack）。
> - **ii 接循环**：body `run_global_refresh_cycle` 在 `refresh_global_relation_predictor`
>   后、`refresh_global_reasoning` 前，遍历活跃 slot 调 `tick_slot_cross_layer`（关系图已刷新）。
> - **iii 持久化 snapshot v49→v50**：per-slot 跨层 weights 作为**独立块**（不进 SnapshotSlot，
>   避免波及 8 处手构造）写入 `SnapshotData.slot_xlayers`。version>=50 门控。
>   `tests/vpc_cross_layer_persist.rs` 1绿：学到的 source_slots/weights/bias 经 write→read
>   完整保留 = **"重要记忆=关系规则,死后保留"支柱立起（R1 核心）**。
> - 回归：B1 变更零回归（85 套件绿）；残 3 失败为既存 real_phot_adapter(vision v2)问题，已切出。
> - 顺带修：arch map snapshot 版数 v49→v50。
| **B1-4（接 C1/语言）** | 涌现 vital_weight（对象因预测续命获权重）；语言切分/接地 | 接 C1 阶段2 / 语言 |

### 【B1-1 落地状态 2026-05-30 — TDD 完成】
代码已落地，9/9 测试绿，机制5 核心首次被代码证明：
- **step1**（`tests/vpc_prior_injection.rs` 4绿）：新增 viewport `prior_signal` 通道
  （`push_prior`/`read_prior`，per-axis AtomicU64 bit-pack f64）；`predictor.rs:97`
  预测式加 `+ external_prior`（prior 只进 predicted_value，**不进 used_x0/x1**，
  故 a/b 梯度永不见 prior = 守无BP红线）。验证：prior 平移预测、降低 err、不污染系数。
- **step2**（`tests/vpc_cross_layer.rs` 2绿）：新增 `src/vpc_cross_layer.rs`
  `CrossLayerPredictor`：稀疏源轴 → 预测目标轴 err（含 bias 项防噪声肩代偏置）→
  本地 Widrow-Hoff 更新自己的 weights（无链式、无回传）→ 写 prior 回视口。
  合成双轴耦合验证：MSE 10倍下降（0.0008→0.00008）、w_b→0.25 而 w_noise→0.008、
  **不写低层 a/b（无BP）**。
- **关键合规更正**：早期测试主张"prior 不污染系数"是错的（且假绿）。正确的无BP判据 =
  "高层不直接改低层系数 + 无链式回传 + 每层只用本地 err"——已按此重写测试。
  机制5 原文"prior→间接降低 err"明确授权 prior 经 err 影响低层学习（= 戴眼镜适应世界，
  非伸手改神经）。

### 【step3 诚实状态 — 部分已证，余下并入 B1-3】
- ✅ 已证："噪声 weight≈0"（step2 第3断言：w_noise 是 w_b 的 1/30）。
- ⏳ 待 B1-3："噪声轴长不出 active 关系边 → 自动不入 source 集合"。当前 CrossLayer 的
  source 是**手工传入**的；spec 设计的"由 global_relations active 边自动门控"需要
  **slot↔轴桥接**（关系是 slot 间的、err 是轴上的），归入 B1-3 多对象阶段，避免返工。
- 即：当前 B1-1 在**轴级**证明了机制核心；关系门控的**对象级**自动排除留 B1-3。

---

## 8. 验收证据（合规底线）

1. **无 BP**：高层只更新自己的 src_weights（本地），从不改低层 a/b 系数。
2. **无全局损失**：每个 CrossLayer 任一 tick 只读稀疏 src 集合（关系门控），非全轴矩阵。
3. **无跨 predictor 直接通信**：err 读视口、prior 写视口，全程经视口。
4. **prior 只进预测不进系数**：`pred + prior`，a/b 更新式不含 prior。
5. **机制5 生效**：合成耦合中 target err 接 prior 后显著下降。
6. **噪声拒绝**：无关系边的噪声轴被自动排除，prior 恒 0。
7. **稀疏性可验**：任一时刻每个 CrossLayer 的 src 集合大小 ≪ 总轴数。

---

## 9. 与最高原则的关系（合规性核验）

**核验结论（2026-05-30）：涌现稀疏跨轴 B1 不违反最高原则与设计理念，可接受。**
逐条核对"不允许的事"：

| 禁止项 | 是否触犯 | 依据 |
|---|---|---|
| BP/链式法则 | 否 | 高层只更新自己 src_weights（本地 delta），梯度不回传源轴 |
| 跨 predictor 直接通信 | 否 | err 读视口、prior 写视口，全程经视口 |
| 全局损失 | 否 | 每高层有自己的本地目标，无统一损失 |
| 直接改 P_low 系数 θ | 否 | prior 只进 pred 加性输入，a/b 更新式不含 prior |
| batch/epoch | 否 | 每 tick 连续 |
| 涌现功能原则 | **强对齐** | 连接由关系图谱涌现门控，非预先写死 |

**全连接 vs 稀疏涌现（论证已更正，见 §2.2）**：

| 维度 | 全连接 | 涌现稀疏（本 spec） |
|---|---|---|
| 是否 BP | 否（单层宽回归=LMS，非BP） | 否 |
| 涌现原则 | **违反**（预先写死密集结构） | **符合**（按关系证据长连接） |
| 噪声 | 强行拟合 | 天然排除 |
| 像生命 | 否（死结构） | 是（按相关性长突触，可增删） |

> 全连接不是"违反最高原则"，而是"违反涌现原则 + 拟合噪声"。
> 涌现稀疏更合涌现理念、更拒噪声、更像生命——大脑按相关性长突触，非全连接。
> **真正的 BP 红线在"层叠+prior链"（见 §2.2b），v0 用单层高层护栏规避。**
