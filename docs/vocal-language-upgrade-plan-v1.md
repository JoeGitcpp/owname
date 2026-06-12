# 发声 / 语言能力 — 彻底升级方案 v1

> 状态：草案 · 2026-06-12 · 作者：Joe + Fable
> 范围：`bionic_v0`（纯 CPU Rust 活体）发声自监督闭环 → 序列层 → 语言种子
> 前置阅读：`consciousness-stream-architecture.md`、`docs/axioms/2026-04-20-new-axioms-v1.md`、`docs/improvement-prompts.md`

---

## 0. 这份文档为什么存在

体到现在学不会说话，触发了对整体设计的怀疑。本次做了一轮代码 + 文档 + 数据的交叉体检，**核心结论是：可疑的不是设计方向，而是「把『说话』当成当前架构的近期目标」这一预期错配**。预测编码内核扎实、值得继续；但「说话」本质是**时间上的、分层的序列问题**，而当前机器是为**静态多尺度空间预测**设计的——差了一个时间维度。这不是想错了，是目标和机器差了一层。

同样重要的发现：**现有设计文档严重滞后于代码（stale）**。多处被文档/直觉判定为「坏掉/没接」的机制，其实早已修复并接进活体循环。拿过时的地图看一个已经前进的系统，是产生「怀疑自己」的一大来源。本方案的第一原则因此是：**先核实真实现状，不重建已存在的东西。**

---

## 1. Stale 文档勘误表（本次核实结果）

| 旧说法（文档/直觉/初轮体检） | 实际代码状态（已核实） | 证据 |
|---|---|---|
| 声学误差没有回流到运动器，运动器只是奴隶式跟随 `pitch_hint` | **错。** 已有真实爬山伺服 `vocal_servo`，每 ~90ms 拿自听形状误差修正发声，并已接进活体循环 | `vocal_servo.rs`；`viewport.rs:4556-4570` |
| 学习误差用的是粗的 2 标量代理，造成「虚假收敛」 | **已修。** 当 `self_voiced` 时，learning_gap 用 64-mel 共振峰级形状距离 | `viewport.rs:4538-4550` |
| `dual_drive` 算了但零下游消费者（A1「悬空动态」） | **已接。** 依恋驱动经 `bond_modulated_region_b` 调制 region-B 发声 | `viewport.rs:4579-4583` |
| `semantic_readiness` / `symbolizer` / `concept_id` 仍在主循环，违反 2026-04-20 语言公理 | **已清。** 全代码库已无任何出现 | `grep` 全树 0 命中 |
| 发声器只有 3 个自由度（pitch/resonance/noise） | **部分错。** `VocalOrganState` 已含 F1/F2/F3 + voiced_ratio + ADSR 包络（`begin_voicing_v2`），但**活体路径未启用**，仍走 3 标量的 `begin_voicing`/`adjust_voicing` | `vocal_organ.rs:76-94`；`viewport.rs:3970,4231,4671` |

> **行动项**：本方案 Phase 4 会把上述勘误回写进 `consciousness-stream-architecture.md` 与 `improvement-prompts.md`，让地图追上领土。

---

## 2. 真正的瓶颈（已逐条核实）

| # | 瓶颈 | 性质 | 严重度 |
|---|---|---|---|
| B1 | 遥测显示闭环 **0 次触发、声学拟合全 0**（`runtime_diag_20260611/vocal_diag.csv`）。学习/伺服路径依赖 `self_voiced`，闭环路径依赖 `self_match>0.85`（≈22ms 窗，几乎打不开） | 信号没端到端跑通；疑点：育婴走参数化喂养 → `target_mel64` 全 0 → 退回粗 16-band；或自听根本没发声 | 阻断级 |
| B2 | 活体只用 3 标量发声路径；带共振峰的 `begin_voicing_v2` 休眠 | 运动天花板：发不出辅音、元音也糊 | 高 |
| B3 | 伺服是 (1+1) 坐标爬山、90ms 一步、只调 2 标量，目标还在动 | 能跟稳态长音，跟不上 ~100ms 的音节 | 高 |
| B4 | **没有时间/序列层**：伺服优化「此刻」；`vocal_memory` 存孤立原型（10s 衰减、不记顺序）；predictor 单步 AR(2) | 真·架构缺口——「学不会说话」的根 | 阻断级（对语言） |
| B5 | `master_learned_bias` 自 6/3 未更新；训练池空 → 每个访客从同一静态点重启 | 体没有在持续成长 | 中 |

---

## 3. 设计约束（全程遵守）

源自项目公理，方案不得违反：

- **本地学习、在线、无反传**：不引入 BPTT / 批量 / epoch。
- **无奖励信号、无正负判断**：只有预测误差。资格迹是「无反传时间信用分配」的神经科学正解，天然契合——它不是奖励，是把已发生的预测误差沿时间回溯归因。
- **涌现优先**：功能是整合基质的涌现现象，不写「脚本化行为」。
- **本体纯净**：人类语言标签不进 body 内部状态；语言是后置的绑定现象，不是先验模块。
- **占用约束**：纯 CPU。序列层的资格迹为「每变量一个衰减标量」，固定大小（几十 KB 量级），**不随回溯时长增长**——刻意避开「存整段历史做反传」那条吃显存的路。

---

## 4. 升级路线（5 期 · 每期带 go/no-go 验证门）

> 纪律：上一期没**亲眼看到信号**，绝不进下一期。呼应 `consciousness-stream-architecture.md` 的 phase-by-phase validation 风险缓解。

### Phase 0 — 确诊与遥测（1–2 天，不改架构）
**目的**：动手术前先让信号可见，停止盲调。
- 每 tick 打点：`self_voiced`、`shape_dist` 实值、`target_mel64` 是否全 0、伺服 `pitch_offset/resonance_offset` 轨迹、闭环 gate 各分量（organ.phase / breath_drive / since_last_vocal）。
- 跑一个短育婴诊断，对齐 `vocal_diag.csv`。
- **预期**：定位 B1 断点。最可能是 `target_mel64` 在参数化喂养路径上全 0（见 `vocal_target.rs:16` 注释），导致 shape_dist 退回粗 band；次可能是自听 PCM 没真正进 `observe_self_vocal_pcm`。
- **门**：能指着说出「信号断在这一环」。✅ 才进 Phase 1。

### Phase 1 — 通信号 + 解除运动天花板（3–5 天）

> **第一刀已落（2026-06-12 · 接通断点）。** 改动（全部纯加法、零行为破坏、`cargo build` + 全发声测试套件绿）：
> - `viewport.rs`：新增游标 `last_imitation_closure_t_us`；在 `tick_vocal_drive_field` 的 region-B 内，当 `self_voiced` 为真时**限速（250ms）地驱动一次 evidence 闭环**——把一直在跑却从不触达 sink 的伺服/自听路，接进现成的诚实门控 efficacy/成熟度机器。
> - 放宽 self-play 门控：新增 `VOCAL_SELF_PLAY_MATCH_MIN=0.5`（`lib.rs`），把 `self_match>0.85`（≈22ms 窗）改为 ≈75ms 窗，让独自咿呀也能 bootstrap。诚实门控（`self_bands>1e-6` + fit 加权）**未动**。
> - 验证：`tests/phase0_vocal_loop_diagnosis.rs::phase1_servo_path_lifts_efficacy_and_maturity`——**仅靠 `tick_vocal_drive_field`、零手动闭环**，12s 内产生 47 次 evidence 闭环，efficacy 0→**0.107**、成熟度 0→**0.020**（之前两者恒 0）。诚实门控回归测试 `closures_without_heard_self_credit_nothing` 仍通过。
> - 诚实留账：efficacy 现在**会爬**，但 shape_dist 仍≈0.58 不降（B3 伺服太弱），即「efficacy 在追踪一个大体不变的拟合度」，而非「身体真的越发越准」。让身体真正逼近目标，仍需本期剩余项（启用 v2 共振峰 + 强化伺服）与 Phase 2 序列层。
>
> **真实育婴管线验证（2026-06-12 · headless nursery_feeder，全新起步）。** 本机跑 `node gateway/nursery_feeder.mjs --body …/release/body --runtime /tmp/aborn-nursery-phase1 --duration 360`（无声卡/无麦，纯结构化重复课程）。~2 分钟遥测：`closures_with_evidence` **0→511**（原始症状是恒 0）、`vocal_efficacy` **0→~0.40**、`developmental_maturity` **0→0.064**、`region_b_ticks` 0→967、`self_voiced=1`。附带确认：`target_mel64_sum`=6–9（喂养器**有**喂精细指纹 → B1a 不是这条路的瓶颈，病根确为「两路断开」，修复打中正确靶）。`shape_dist` 仍≈0.38 平（B3 待办）。**第一刀 live 验证通过。**

**目的**：让伺服吃到精细误差，并给它一个值得塑造的运动器。
- 修 Phase 0 找到的断点（很可能：让育婴喂养也带上 64-mel 指纹，使 `target_mel64` 非零）。
- **启用 `begin_voicing_v2`**：活体发声路径从 3 标量切到 F1/F2/F3 + voiced_ratio + noise + ADSR。组织结构已支持，只差接线。
- 伺服从「pitch/resonance 2 维」扩到「共振峰空间」爬山。
- **预期**：`vocal_efficacy` 第一次真正爬升；体能跟读稳态元音与音高，元音轮廓变清晰。
- **门**：给稳态 /a/，efficacy 上行 + shape_dist 下降。**这一步即可回答「机制到底动不动」。** ✅ 才进 Phase 2。

> **第二刀已落（2026-06-12 · 升级合成器 + 共振峰伺服）。** 改动：
> - `vocal_synthesis.rs`：`synthesize()` 改读 organ 的**显式** F1/F2/F3（原先 F1/F2 由 resonance 内部锁在一条 1D 线上）；旧 3 标量路径通过 `vocal_organ::formants_from_resonance` 写入同一基线，**声音不变**。
> - `vocal_organ.rs`：`begin_voicing`/`adjust_toward` 把 resonance→(F1,F2) 写进显式字段。
> - `viewport.rs` + `lib.rs`：声学伺服的两个爬山旋钮从「pitch / resonance(1D线)」**重定义为独立 (F1,F2) Hz 偏移**（`F1_SERVO_HZ=600`、`F2_SERVO_HZ=1300`），在 outlet 发声后施加到 organ 共振峰——让身体能离开 1D 线进入 2D 元音空间。全测试套件 + 合成单测绿。
>
> **结果（诚实）：能力成立，但不充分。**
> - ✅ **静止目标**：确定性测试 `phase1_formant_servo_descends_on_held_offline_target` 证明伺服在可达的离线目标上把 shape_dist 从 0.050 降到 0.028（Δ−44%）——共振峰控制真的在 steer。
> - ❌ **真实育婴（移动目标）**：200s 跑 shape_dist 全程均值 0.484→**0.480（没降）**，前 1/3→后 1/3 反而 0.445→0.524。病因实锤：育婴课程每几秒切换元音，而伺服 90ms 一步的慢爬山**还没收敛目标就变了**，committed 偏移一直在追。
> - **结论**：升级合成器是**必要**的（加了 2D 能力、零回归、静止可降），但**逐瞬爬山控制在非平稳目标上撞墙**——这正是 Phase 2 序列层要解决的核心。下一步杠杆 = Phase 2（时间/序列），或一个能在单元音保持时长内收敛的「快伺服」作为过渡。

### Phase 2 — 序列层（核心架构升级，1–2 周）
**目的**：从「匹配此刻」升级到「按顺序模仿音节」。本期是真正的新组织。

> **增量 1 已落（2026-06-12 · 目标键控运动召回）。** 数据驱动定优先级：第二刀证明伺服能匹配**静止**目标，但在育婴的**重复**目标上每次都冷启动重爬。增量 1 = 内容寻址记忆，存「目标 64-mel 指纹 → 伺服(F1,F2)偏移」，目标复现时 warm-start 伺服。
> - 新增 `vocal_target_recall.rs`（`recall`/`store`，cap 16，最近最少用淘汰，5 单测）；`vocal_servo.rs` 加 `warm_start`/`reset`；`viewport.rs` 在伺服块按目标(再)出现 warm-start、好拟合时 store（含 `last_recall_idx` 游标 + `recall_entries` 遥测）。
> - 验证：`phase2_recall_speeds_up_recurring_target`——目标 A 首次出现 early shape_dist=0.130，学过后复现=**0.046（↓65%，约快 3 倍）**，且 A/B 两目标正确分桶（2 条目）。全测试套件 + 55 lib 单测绿、零回归。
> - **关键校准发现**：`spectral_shape_distance_mel64` 尺度很小（清晰不同元音仅 ≈0.06 隔开）→ 匹配半径收紧到 0.04；且育婴 self↔target 距离有 **≈0.48 的高地板，由音质/声源失配主导**（喂养器解析合成 ≠ 身体声门合成），共振峰伺服关不掉它 → 存储改为「每目标保留最优」而非绝对 fit 门槛。**推论**：要让育婴 shape_dist 真正大降，下一个杠杆是缩小**合成音质差**（让身体声源更接近被模仿者），而非更多控制维度。
> - **活体验证（2026-06-12 · headless nursery 200s）**：`recall_entries` 0→**16**（1 分钟填满）——召回在真实管线确实填充；efficacy 0.285、maturity 0.488 与上轮持平、零回归、跑满无崩溃。shape_dist 均值 0.477（三轮 0.484/0.480/0.477，基本持平）——**音质地板未动，符合预期**。
> - 待办：召回记忆的持久化（snapshot/checkpoint），目前仅在体内 RAM、重启丢失。
>
> **🔑 战略转折（增量 1 之后逼出的真结论）**：已依次补齐 断点→控制维度(2D共振峰)→复现记忆(召回)，每块机制层面都验证成功，但**活体 shape_dist 始终 ~0.48 纹丝不动**。瓶颈已易主——**当前拱心石是误差度量/音质空间**：shape_dist 把「可控的共振峰/元音结构」与「不可控的声源音色」混在一起，地板来自后者、盖住了前者。**最高价值的下一步不是更多控制/记忆，而是让学习误差落在可控空间**：做一个音高/音色归一化的形状距离，只衡量共振峰模式匹配。这样共振峰学习进步首次可见、召回与伺服成果首次能转化为 shape_dist 下降，且更贴合公理（奖励可控预测结构，而非惩罚不可约失配）。→ 列为 **Phase 2 增量 2 候选**（或独立的"度量重构"小项，先于序列/资格迹）。
- **资格迹（eligibility trace）**：每个发声动作留一道衰减尾迹；延迟到来的声学误差顺着尾迹回溯归因给过去几百毫秒的动作。本地、在线、固定内存。
- **顺序记忆**：`vocal_memory` 从「孤立原型」升级为「带转移概率的序列」——记住「这一下之后通常跟那一下」。
- **多步预测**：predictor 从单步 AR(2) 扩出「预测下一个音素」的能力（多时间尺度，可先做 2 步）。
- 新增可调旋钮：尾迹衰减半衰期、回溯窗长、转移记忆容量。
- **预期**：体开始模仿**两段式**音（如「妈—妈」）；听到前半段能预期后半段。
- **门**：能复现一个 CV 或双音节轮廓，且 efficacy 比 Phase 1 更稳。✅ 才进 Phase 3。

### Phase 3 — 巩固与持久化（3–5 天）
**目的**：学到的东西跨天存活，体真实长大。
- 修 `master_learned_bias` 不更新 / 训练池空（B5）：会话学习能 harvest/merge 回 master。
- 给已学音素加「巩固 / 重放」抗衰减机制，别 10s 即忘。
- **门**：重启后保留已学音素；master 文件随会话真实增长。

### Phase 4 — 架构债清理与文档勘误（2–3 天）
**目的**：让文档追上代码。
- 把第 1 节勘误表回写进 `consciousness-stream-architecture.md`、`improvement-prompts.md`。
- 复核 `semantic_readiness`/`symbolizer` 确已无残留（当前 grep 为 0），在文档中标注「已完成」。
- 更新里程碑状态表。

---

## 5. 诚实的天花板

做完 Phase 0–4，现实预期上限：**应答式咿呀 → 跟读音高/元音 → 模仿音节 / 双音节轮廓**。

到「能说词、有语义」还需要**符号接地层**（声音 ↔ 对象 ↔ 动作的跨模态绑定），属 Phase 5+。建议等序列层验证后再设计——不在此刻许诺。原因：语义不是匹配出来的，是多模态预测误差在「对象层」之上反复共现绑定出来的，需要对象/关系层先成熟（见 2026-04-20 语言公理：object-first）。

---

## 6. 里程碑与验证指标

| 里程碑 | 指标 | 验证方式 |
|---|---|---|
| M0 信号可见 | 能定位断点 | ✅ 已达成 2026-06-12（见 §7.5）：病根=伺服路与 efficacy/时钟断开 |
| M1 机制活着 | 稳态元音 efficacy 单调上行、shape_dist 下降 | 🟡 2026-06-12：efficacy 已上行(0→0.3)；shape_dist 静止目标可降(0.050→0.028)但移动目标(育婴)不降——撞非平稳墙，待 Phase 2 |
| M2 运动充分 | 能产生可区分的多元音 + 起码一类辅音特征 | 自听 mel64 聚类 |
| M3 序列涌现 | 复现双音节轮廓；前半段→后半段预测误差下降 | 序列诊断 |
| M4 记忆持久 | 重启保留音素；master 随会话增长 | 跨会话对比 |

---

## 7. 风险与缓解

| 风险 | 缓解 |
|---|---|
| Phase 0 发现信号其实通、瓶颈另在他处 | 这正是 Phase 0 的价值——按数据重排后续期，不沉没成本 |
| 序列层引入不稳定 / 振荡 | 资格迹有界 + 衰减；先单维资格迹验证再扩；保留 Phase 1 配置可回滚 |
| reserve 经济在长育婴会话耗尽（历史问题） | 已有 survival gate（`viewport.rs:4595+`）；Phase 2 前先确认 duty-cycle 稳定 |
| 运动空间扩大后伺服维度灾难 | 伺服按维度分组、分时爬山；或后续用资格迹替代纯爬山 |
| 改动撞 stale 文档造成误判 | 第 1 节勘误表为单一事实源；Phase 4 收口 |

---

## 7.5 Phase 0 确诊结果（2026-06-12 · 已完成）

确诊手段：① 给活体加 Phase-0 遥测（`viewport.rs` 的 `VocalLoopDiag` 扩展 + `bin/body.rs` 的 `vocal_diag.csv` 新增 13 列，全部纯观测）；② 写确定性集成测试 `tests/phase0_vocal_loop_diagnosis.rs`，无麦克风/网关，按 body 主循环时序逐 tick 驱动真实闭环并读 diag。两测试均通过，数据如下（喂正确 mel64 目标，80 tick ≈ 1.6s）：

```
tick  region_b  self_voiced  shape_dist  servo_p  servo_r  tgt_mel64Σ  efficacy
   0         1         true      0.6204    0.060    0.000      3.1046    0.0000
  40        41         true      0.6063    0.000    0.000      3.1046    0.0000
  79        80         true      0.5880    0.000    0.120      3.1046    0.0000
```

**核心结论（4 条，按重要性）：**

1. **闭环没死。** 喂正确 mel64 目标时，region-B 每 tick 都跑、`self_voiced=true`（efference copy 正常）、shape_dist 是真实的 64-mel 形状误差、伺服在动。生产 `vocal_diag.csv` 的「closures=0 / fit=0」是**遥测盲区**——那两列只在闭环 sink 写，而 sink 几乎不触发。**不是机制坏了。**

2. **🔴 真正的病根：两条路断开。** efficacy 在 80 tick 里**恒为 0.0000**，尽管每 tick 都 self_voiced + 有真实误差。原因——存在两条彼此不通的路：
   - **伺服/模仿路**：靠 `self_voiced` 运行，会操纵运动器、算 shape_dist——但**不更新 `vocal_efficacy`，也不推发育时钟**。
   - **闭环/efficacy 路**：是**唯一**能动 efficacy + 成熟度的路——但门控 `self_match>0.85` 要求距上次发声 **<22.5ms**（guard=300ms，见 `lib.rs:394`），50Hz 下基本只有发声后那一个 tick 能开。→ 几乎永不触发 → **efficacy/成熟度被冻在 0**。
   - 即 memory 里那条「服务器 mat≈0.9999 是幻觉成熟度」之外，真相是：**能学的路（伺服）跟学习信号（efficacy/时钟）是断开的。** 这才是「学不会、时钟不走」的拱心石，且与喂养方式无关。

3. **伺服太弱（B3 实证）。** shape_dist 1.6s 内只从 0.62 降到 0.588，servo 偏移在 0/0.06/0.12/0.18 间抖动、不收敛——(1+1) 坐标爬山闭不掉真实的元音↔元音差距。

4. **参数化喂养饿死 mel64（B1a 实证）。** 走 `observe_vocal_target`（无原始帧）时 `target_mel64Σ=0` → shape_dist 静默退回粗 16-band。若育婴喂养器走这条路，即是症状之一。

**对方案的影响：Phase 1 优先级重排。** 首要不再是「修喂养」，而是 **把伺服路与 efficacy/发育时钟接通**（让 self_voiced 路上的真实声学拟合直接喂 efficacy EMA，不再独占于 22.5ms 闭环窗），并放宽/重做 `self_match` 门控。其次才是启用 v2 共振峰路径与强化伺服。

---

## 8. 下一步

建议顺序：**本文档（已落）→ Phase 0（加遥测跑短诊断）**。Phase 0 是把「我是不是想错了」变成「我看见它断在这里、也看见它能学」最快的一步。每期完成后回写本文档的里程碑表。
