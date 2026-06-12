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

### 增量 2：分支 seed 补全（低风险纯增量）
`server.mjs` seed 循环加主线声学 5 件（`master_phoneme_memory.bin` 等），让分支**基于当前主线水平**起步。

### 增量 3：分支 harvest 回收声学件 + 主线摄取
- 分支结束：把声学件复制进 `training_pool`（沿用 learned_bias 路径）。
- 主线/合并器周期性 **merge 摄取** pool（调用增量 1 的 merge）。建议：扩展现有 pool-merge 为一个 Rust `merge_artifacts` 工具（merge 逻辑随数据结构在 Rust），网关周期调用。

### 增量 4：运维与健壮性
- 主线自动常驻（gateway 启动时 spawn `nursery_feeder --forever --master`，崩溃重启 / systemd）。
- 重置被污染的 master maturity（一次性，从干净新生儿重起谱系）。
- 并发文件锁（flock）包住 master/pool 写。

### 未来（Joe）：账号登录 → 长期持久实例 → 按 AI 持久记忆（接 `memory_store.mjs`），沟通验证后开放。

---

## 5. 设计原则
- **合并 > 覆盖**：N 分支 → 1 主线，证据相加/取优，绝不末位写者胜。
- **merge 逻辑随数据结构在 Rust**（可单测），网关只做编排/复制。
- **主线累积通用技能，个人件按用户**（不污染主线身份）。
- **回收前评估**（沿用 `training_pool.mjs` 质量门）避免坏会话污染主线。
