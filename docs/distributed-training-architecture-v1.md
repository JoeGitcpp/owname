# 分布式训练架构 — 主线生命 + 分支 + 回收 v1

> 状态：草案 · 2026-06-12 · 作者：Joe + Fable
> 愿景：服务器常驻一条**主线生命**持续成长；aborn.live 访客实例是**分支**，开新分支基于**当前主线水平**播种；对话结束后把分支的成长**回收**进主线。
> 未来（Joe 自建）：账号登录、长期持久实例、按 AI 的持久记忆（沟通验证后）。

---

## 1. 心智模型

```
              ┌─────────────────────────────┐
              │     主线生命 (mainline)      │  常驻、持续成长
              │  master 学习产物 = 它的"水平" │
              └───────┬─────────────▲────────┘
            seed(播种) │             │ harvest+merge(回收合并)
                       ▼             │
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ 分支A     │ │ 分支B     │ │ 分支C     │  访客对话实例(短暂)
        │(访客对话) │ │(访客对话) │ │(访客对话) │
        └──────────┘ └──────────┘ └──────────┘
         对话结束→把最终学习产物写入 pool→主线定期摄取
```

- **主线**：一个长期运行的 body（现状最接近：`nursery_feeder --forever --master`）。它的"水平"= 一组 master 学习产物文件。
- **分支**：访客一连接就 spawn 的短暂 body（`server.mjs`），seed 自主线，对话结束销毁。
- **回收**：分支结束 → 最终产物进 `training_pool` → 主线定期**合并**摄取（不是覆盖）。

---

## 2. 什么属于主线，什么属于个人（关键划分）

| 产物 | 归属 | 理由 |
|---|---|---|
| `learned_bias.bin`（偏置/maturity/vocal_efficacy） | **主线**（共享技能） | 通用发育水平，应累积 |
| `phoneme_memory.bin` | **主线** | 声素原型库=怎么发音，通用 |
| `vocal_memory.bin` | **主线** | 运动原型，通用 |
| `vocal_target_recall.bin` | **主线** | 目标→发声偏移，通用发声技能 |
| `snapshot.bin`（跨层预测器） | **主线** | 感知/抽象技能，通用 |
| `familiar_objects.bin`（认识谁） | **个人**（按用户） | 关系，不该把访客A的脸合进全局主线 |
| `vital_core.bin` / `self_model.bin` | **个人/短暂** | 生理/自我状态，按用户或重置 |

> 现状已把"个人"三件按用户目录存（`server.mjs`），方向正确。本架构只需把**主线那 5 件**的双向流补全。

---

## 3. 现状 EXISTS / MISSING（探查 2026-06-12）

| 环节 | 状态 | 证据 |
|---|---|---|
| 分支生命周期（创建/结束/清理） | ✅ | `server.mjs` 实例与会话管理 |
| 分支 seed `learned_bias` | ✅ | `server.mjs` seed 循环 |
| 分支结束 harvest `learned_bias`→pool→EMA 合并 master | ✅ | `training_pool.mjs` merge_pool |
| 主线候选（nursery_feeder --forever --master） | ⚠️ 存在但需手动起、未集成 | `nursery_feeder.mjs` |
| 分支 seed **声学 5 件** | ❌ 只 seed learned_bias+个人件 | 访客每次从零学发声 |
| 分支 harvest **声学件** | ❌ 完全不回收 | 访客发声学习全丢 |
| `vocal_target_recall` 在任何 harvest/seed | ❌ 全缺 | 本会话新建的技能孤立 |
| `snapshot` 持久化/回收 | ❌ 全缺 | 跨层抽象每会话重置 |
| 多分支并发回收 = 合并而非覆盖 | ❌ copyFileSync 覆盖、无锁 | 末位写者胜、可丢学习 |
| 主线自动常驻 | ❌ 独立 CLI，崩溃不重启 | — |
| master maturity 污染（幻觉 0.9999） | ❌ 待重置 | 见 memory improvement-prompts-rollout |

---

## 4. 增量计划

### ✅ 增量 1（已落 2026-06-12）：原型类 merge 内核
分布式训练的工程核心——多分支学习如何**合并**成一条主线（不是末位覆盖）。
- `VocalTargetRecall::merge`：逐条 `store`，每目标保留最优拟合、并集去重、尊重 cap。
- `VocalMemoryState::merge`：邻近原型 stability/reuse **相加**（多分支独立确认的手势最稳）、novel 并入、按 stability 截断 8。
- 单测：reinforce 共享 / 吸收 novel / 不被更差分支拉低 / 尊重 cap。
- 待补：`PhonemeMemory::merge`、`snapshot` 合并（EMA，同结构）、`learned_bias` 已有 EMA。

### ✅ 增量 2（已落 2026-06-12）：分支基于当前主线水平
- `nursery_feeder.mjs`：`MASTER_ARTIFACTS` + 重启 seed 列表补上 `vocal_target_recall.bin` 与 `snapshot.bin`（此前孤立，主线每次重解）。主线现在 harvest 出完整 master_ 产物集。
- `server.mjs`：新增 `MAINLINE_VOCAL_FILES = [phoneme_memory, vocal_memory, vocal_target_recall, snapshot]`，分支 seed 时从 `master_<name>.bin` 复制进 runtime（fallback MASTER_RUNTIME），与 PERSONAL_FILES（按用户）分流。
- 验证：① 跑 `nursery_feeder --master` 35s → 产出 `master_vocal_target_recall.bin`(9条目) + `master_snapshot.bin`(12MB) 等；② 用这些 master 文件播种一个全新 body → 首行 **recall_entries=9 / maturity=0.10 / efficacy=0.38**（继承主线水平，非从零）。40 个网关实例/会话测试绿。
- ⚠️ 线上落地需部署：`deploy_aborn.sh` 同步网关。

### ✅ 增量 3（已落 2026-06-12）：回收（分支→pool→主线 merge 摄取）
- **补齐 merge 内核**：`PhonemeMemory::merge`（按 rehearsal 证据加权合并匹配原型、novel 填空槽/满则换更弱者、不回归；单测：reinforce 共享/吸收 novel/identical 不重复）。recall+vocal_memory 已于增量 1 做。snapshot 定为 **seed-only**（主线自养，分支继承不回合——EMA 合并 12MB 结构化跨层状态风险高、短会话漂移不值得）；learned_bias 走 Node EMA。
- **`bin/merge_artifacts`**（Rust 工具）：`merge_artifacts <master_dir> <pool_dir>`，逐类型读 `master_<name>.bin` + 合并所有 `*__<name>.bin` pool 文件 + 写回 + 消费 pool 文件。**单线程**运行（网关周期 tick）→ 分支只写 pool、绝不与 master 竞争。
- **`server.mjs` 接线**：① 分支结束(过质量门)把 `MAINLINE_HARVEST_FILES`(recall/vocal_memory/phoneme) 复制进 pool，命名 `{ts}_{sid}__<name>.bin`；② 周期(MERGE_INTERVAL_MIN)在 merge_pool 之后 spawn `merge_artifacts` 摄取。
- **验证**：merge 单测全绿；工具端到端(ingest+消费 pool)；72 网关测试绿；同课程分支合并正确**去重**(9→9，保留最优不膨胀)，distinct-target union 增长由单测保证。
- **意义**：闭环成立——seed(增量2)→学→harvest 进 pool→merge 进 master→下个分支 seed 更强。同课程下主线 recall **收敛+逐分支改进校正**(非无限膨胀)。
- ⚠️ 线上需部署：网关 + `merge_artifacts` 二进制(服务器 cargo 重编)。

### ✅ 增量 4（部分已落 2026-06-12）：运维与健壮性
- **主线自动常驻**（已落）：`server.mjs` 在 listen 后 env-gated（`ABORN_MAINLINE=1`）spawn `nursery_feeder --forever --master`，崩溃**指数退避重启**（2s→cap 60s，健康运行后复位）。冒烟验证：日志 `[mainline] spawning continuous life`，主线 runtime 创建。
- **原子 harvest 写**（已落）：分支 harvest 到 pool 改为写 `.tmp` 再 `renameSync`，使周期 merge 永不读到半写文件而静默丢弃。
- **重置污染 master maturity**（脚本已落，待你在服务器跑）：`gateway/reset_master_maturity.mjs [master路径]`——清零 learned_bias 末 f64（developmental_maturity），带时间戳备份，biases/efficacy 保留。验证：0.10→0。
- 并发锁：周期 merge 单线程（主竞争已避免）+ 原子 rename（pool 竞争已避免）；个人文件 same-user 并发写仍 last-writer-wins（低 severity，下次会话自愈），暂不加 flock。

### 服务器部署清单（Joe 执行）
1. `git pull`（已 push 到 main）。
2. 服务器 `cd bionic_v0 && source ~/.cargo/env && cargo build --release`（编出新的 `body` + `merge_artifacts`）。
3. `deploy_aborn.sh` 同步网关（参 server-deploy-ops）。
4. **一次性**：`node gateway/reset_master_maturity.mjs <master_learned_bias.bin>`（清幻觉成熟度）。
5. 启动网关时设 `ABORN_MAINLINE=1`（开主线常驻）；`ABORN_MERGE_MIN` 控制摄取周期。
6. 跑一段后核对：`master_vocal_target_recall.bin` 等随时间增长/改进，访客新会话 recall>0、maturity 继承。

### 未来（Joe）：账号登录 → 长期持久实例 → 按 AI 持久记忆（接 `memory_store.mjs`），沟通验证后开放。

---

## 5. 设计原则
- **合并 > 覆盖**：N 分支 → 1 主线，证据相加/取优，绝不末位写者胜。
- **merge 逻辑随数据结构在 Rust**（可单测），网关只做编排/复制。
- **主线累积通用技能，个人件按用户**（不污染主线身份）。
- **回收前评估**（沿用 `training_pool.mjs` 质量门）避免坏会话污染主线。
