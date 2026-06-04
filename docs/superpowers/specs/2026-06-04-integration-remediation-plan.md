# 集成补救计划 · Integration Remediation Plan

> 日期：2026-06-04 ｜ 状态：待执行
> 来源：整合审核发现 ① 高斯视觉场没建、② 主动感知是未接线孤岛、④ tier-0 统一半成、⑤ 能力课程治理器是空壳；③ 好奇→学习进度已真成且接线。
> 执行原则：**不敷衍、不临时、不做暂时工程。**

---

## 0. 最高原则（贯穿全程，不可妥协）

### 0.1 涌现红线
- 无语义先验：无 label / 物体类别 / 身份表 / 预训练权重。
- 本地学习、无 backprop（无 autograd / `.backward` / 跨层链式）。
- 一套 tier-0：一切感知经 `viewport.rs::push_observation` 进统一轴流；不另立平行表征。
- 物理先验允许：局部性 / 平移不变 / 多尺度 / 稀疏 / 侧向抑制 / 慢特征。

### 0.2 不做暂时工程（本次新增、强约束）
1. **孤岛不靠"接线充数"修**：不是把死代码塞进 body 让它"看起来连上了"，而是**重建到正确的依附对象上**。
2. **桩与真品不并存**：4-region 深度、双字节表征这类临时件，**替换并删除**，不留着"以后再说"。
3. **"单测绿"不等于"已集成"**：禁止用孤立单测冒充集成。

### 0.3 完成定义（DoD）—— 每个模块必须三条全满足，否则不算完成
- **(a) 被 `body.rs` 主循环真实调用**（不是只在 `lib.rs` 声明 `mod`）。
- **(b) 产物落到 tier-0 轴**（`push_observation`），或被落轴的东西消费。
- **(c) 有一条集成测试**断言"主循环驱动了它、且它影响了下游预测/行为"——不是孤立单测。

> 这三条是对"测试绿≠整合"的结构性根治。任何新模块无 body 调用方 + 无集成测试 = 未完成。

---

## 1. 依赖结构（为什么这样排序）

```
①高斯场(拱心石)
   ├── ②深度：per-Gaussian 逆深度 ρ，依附高斯基元（不是 4 region）
   └── ⑤治理器：①编码器层探针读高斯 splat 重建（不是听觉 hl_pred_mse）
④数字对象统一：不依赖①
④身份统一：依赖①（身份要读视觉高斯参数）
```

执行序：**Phase 0（止血+数字统一，独立）→ Phase 1（建①并接线）→ Phase 2（在①上重建②）→ Phase 3（在①上重建⑤）→ Phase 4（身份并入④b）。**

---

## Phase 0 — 止血与数字统一（不依赖①）

**目标**：测试归零、数字对象单一表征、地基场地清干净。

**动作**
- **AXIS_COUNT 不变量根因修**（不是改测试数字）：先判定两个失败测试（`axis_ranges_..._cover_axis_count`、`predictor_count_matches_axis_count`）是"测试过时"还是"真布局 bug"——若 predictor 向量没随 AXIS_COUNT=39 扩容/轴区间有缺口，那是真 bug，修代码；若仅测试常量滞后，连同不变量一起更新。
- **删过时的 ③ 旧测试** `surprise_pressure_refresh_starts_physical_vocal_write`（及 `relation_pressure_*` 若同样记录被废弃行为），确认 ③ 新测试 `probe_drive_raw_surprise_alone_fires_nothing` 已覆盖正确行为。
- **查修 vocal_runtime 真回归** `recent_body_write_coupled_vib_does_not_overwrite_vocal_target`：定位 AXIS_COUNT/snapshot 改动如何扰动 `vocal_target` 写耦合守卫，根因修（HEAD 上 15/15 通过，确系本次引入）。
- **④a 数字对象单一表征**：让 5 条 CODE 轴投影（`viewport.rs:2096`）成为唯一对象表征；`CodeIngressState` 的 `byte_*` 仅作"瞬时抽取源"喂投影，删除任何把它当独立对象模型消费的下游路径；若某处必须保留，文档证明其为纯派生。
- **既有坏测试** `prolonged_silence_depletes_toward_standby`（HEAD 上也 FAIL，非本次）：诚实处理——根因修，或显式记录为已知既有缺陷（不静默忽略、不松阈值绿）。

**拒绝的暂时工程**：改松测试阈值求绿；双字节表征并存只加一句"projected"注释。

**DoD**：`cargo test` 全绿（被显式 quarantine 的有书面记录）；数字对象只有一套表征且在轴上。

---

## Phase 1 — 建 ① 高斯视觉场（地基，CPU，必须接线）

**目标**：视觉前端从 2×2×20 换成稀疏各向异性高斯场，并真正进 body 循环。

**动作**
- 新模块 `phot_gaussian.rs`：`GaussianField`（~256 基元）、`splat/footprint/residual`、本地闭式更新（μ 朝残差质心 / color 朝残差均色 / opacity 看脚印误差 / Σ 朝残差展布，带最小尺寸下限）、birth/death（照 `claim_slot_for_event` 模式写高斯池分配器）、稀疏 top-k + 侧抑（跨通道除法归一化）+ 慢特征项。**无 backprop**。
- 取代 `phot_camera.rs::region_perception`；高斯参数镜像成 tier-0 轴（`push_observation`）；`lib.rs` 轴布局与 `snapshot` 版本相应升级。
- **接线 `body.rs`**：真实相机路径驱动高斯场，每 tick 跑泼溅-残差-更新。
- 接 `curriculum.rs`/`live_human_nursery.rs`：简单→慢→重复起步。
- SoA 布局、高斯间无顺序依赖（为将来 GPU 移植留形状）。

**拒绝的暂时工程**：把 2×2 region 留着"以后再换"；高斯场只做模块+单测不接 body。

**DoD**：(a) body 驱动；(b) 高斯参数写轴；(c) 集成测试断言主循环产出非平凡高斯场且写了轴；**探针**：课程场景下 splat 重建误差 < 复制上一帧基线、高斯数 ±20% 自适应稳定。

---

## Phase 2 — 在①上重建 ②（深度/主动感知），删除 4-region 桩

**目标**：深度从"4 粗 region 的孤岛桩"变成"per-Gaussian 逆深度，真正进闭环"。

**动作**
- `inv_depth` ρ 成为 `GaussianPrimitive` 的 per-object 参数，用同一本地视差规则 per-Gaussian 更新。
- **接线 `body.rs`**：`push_efference_copy` + `inject_efference_motion_priors` + 深度更新每 tick 真实调用；运动来自 `virtual_neck` 作为课程输入提供器，或真实相机运动。
- **删除** `depth_observer.rs` 的 4-region 实现（桩，按不留临时工程原则移除）；`efference_copy.rs` 的 region 级 prior 注入改为 per-Gaussian。
- 保留"静止/纯旋转→深度不可观测"这条正确行为。

**拒绝的暂时工程**：把现有 4-region depth + 当前从不被调的 `push_prior` 链直接接进 body 充数。

**DoD**：(a) body 驱动深度；(b) per-Gaussian ρ 的运动先验在 body 里真实 fire；(c) 集成测试断言"运动→可观测且 motion 轴预测误差下降 / 静止→不可观测"。

---

## Phase 3 — 在①上重建 ⑤（能力治理器），接进主循环

**目标**：治理器从"自洽空壳 + 听觉冒充视觉探针"变成"真门控、探针读真信号"。

**动作**
- ①编码器层探针改读 ① 的**真实高斯 splat 重建信号**，替换当前语义错配的听觉 `hl_pred_mse/hl_avg_mse`。
- **接线 `body.rs`**：治理器实例化并每周期 `update`；其门控**真实影响**各认知层输出的 feedthrough（下层探针未绿则上层不参与/输出被抑制）。
- 用 `substrate::unit()` 统一数值法（与代码库其余一致）。

**拒绝的暂时工程**：保留听觉 `hl_pred_mse` 当编码器门；治理器继续只在测试里跑手搓合成快照。

**DoD**：(a) body 实例化并驱动；(b) 门控真实改变行为（可观测某层在下层绿前被抑制）；(c) 集成测试用**真实 body run**（非手搓快照）断言冷启动→逐层点亮，且任一下层转红→上层关闭。

---

## Phase 4 — ④b 身份并入统一轴流（依赖①）

**目标**：`familiar_objects` 身份从"独立音频指纹 + 独立 .bin"变成"读统一轴流（含视觉高斯参数）的多模态涌现身份"。

**动作**
- 指纹改为从统一轴流（VIB + PHOT/高斯参数）派生，多模态。
- `.bin` 明确为**纯持久化缓存**（派生自轴态）或并入 snapshot；单一真相。

**拒绝的暂时工程**：只加"派生层"注释而保留独立表征当真相。

**DoD**：(a)/(b) 身份可从统一轴流复算、无独立平行对象模型；(c) 集成测试断言重启后身份从轴态复现，且持久化只是缓存。

---

## 横切守则 · 防孤岛

每个 Phase 的验收**强制 DoD (a)+(c)**：body 真实驱动 + 集成测试断言主循环驱动并影响下游。新模块若无 body 调用方与集成测试，一律退回，不算完成。

## 收敛风险（①的真风险，非工程风险）

纯涌现从生像素在线学习收敛慢——用**稀疏 / 侧抑 / 慢特征**三脚手架 + **⑤ 的能力课程门控**（①绿才开②…）对冲；先小（32²）跑出收敛信号再提分辨率。好奇已接 `INTERO_LEARNING_PROGRESS`（③）避 noisy-TV。

## GPU

保持 CPU-first（①稀疏场稳态便宜，~1e5 像素写/帧）；撞到吞吐墙（CPU 扛不住 30Hz 或多 body 并发）再移植 **wgpu compute**（splat 自定义 shader）。本计划全程 CPU。

---

## 一页摘要

| Phase | 内容 | 依赖 | 关键"不做暂时工程"点 |
|---|---|---|---|
| 0 | 止血：AXIS_COUNT 根因修 / 删过时③测试 / 查修 vocal 回归 / ④a 数字单一表征 | 无 | 不松阈值求绿；不让双字节表征并存 |
| 1 | 建①高斯场 + 接线 body + 写轴 | 无 | 不留 2×2；不做"模块+单测不接 body" |
| 2 | 在①上重建②：per-Gaussian 深度 + 接线 + 删 4-region 桩 | 1 | 不把 4-region 桩接线充数 |
| 3 | 在①上重建⑤：探针读真重建 + 接线门控 | 1 | 不留听觉假探针；不只跑合成快照 |
| 4 | ④b 身份并入轴流（多模态） | 1 | 不只加"派生层"注释 |
