# Self-Model Boundary v0 — 自我模型边界建立设计

> **Status:** FROZEN CANDIDATE
> **Date:** 2026-05-24
> **Parent spec:** [Live Life v0](../2026-05-24-live-life-v0-design.md) Layer 2
> **Scope:** 在 Nursery 教学闭环基础上，让系统建立"自己是谁"的最小边界感知。
> 不新增 body 本体模块，不引入 identity/ego/self/owner/personality 语义结构。

## Boundary

`自我`、`自体`、`归属`、`意识`、`身体` 是外部设计描述词，不是内部 ontology。
Body 内部只存储数值 substrate：correlation、delay、magnitude、match、awareness、agency。
所有字段均可从已有 snapshot 数据重新计算，不依赖外部标签。

禁止引入：
- SelfModelSlot 结构体 / self_id / owner_id
- 自我描述字符串 / 自我标签 / 自我概念
- 情绪模块 / 人格模块 / 欲望模块
- body-side 自我诊断文本输出

---

## 三层自体感知

### 层 1: 自体-非自体边界 (`self_vocal_correlation`)

**数据路径：**
```
vocal write epoch (body 输出声音)
  → VIB consequence window (麦克风收到声音)
  → pairing: (delay_us, magnitude, curve_similarity)
  → self_vocal_correlation = 短延迟 + 高匹配 → "这是我发出的"
```

**数值定义：**
- `self_vocal_correlation`: EMA of `pair_match_score` where pair_match_score = `(1.0 - normalized_delay) * curve_similarity * magnitude`
- `external_vocal_correlation`: EMA of VIB events that do NOT pair with vocal write epochs → "这是外部声音"
- `self_other_separation`: `self_vocal_correlation / (self_vocal_correlation + external_vocal_correlation + epsilon)` → 0=全是外部, 1=全是我

### 层 2: 本体状态感 (`body_state_awareness`)

**数据来源：**
```
phot_health (0=FAULT, 1=OK)
  → phot_fault_duration: 累计故障时长
vib_activity (ambient_vib.event_energy > threshold)
  → vib_quiet_duration: 累计静默时长
allostatic_stability:
  → resource_level, recovery_debt, bearing_capacity
```

**数值定义：**
- `body_state_awareness`: 复合信号，health 低时上升（故障引起自体注意），health 稳时下降到基线
- `fatigue_pressure`: 累积运行时间 + recovery_debt 的单调函数
- `health_surprise`: phot_health 或 vib 状态突变时的瞬时峰值

### 层 3: 行为-后果归属 (`agency_pressure`)

**数据路径：**
```
write epoch count → consequence window count → paired count
agency = paired / (write_epochs + epsilon)
self_caused: 高 agency + 短延迟 → 系统感知到"我引起了这个变化"
other_caused: consequence window 无对应 write epoch → 外部事件
```

**数值定义：**
- `agency_pressure`: EMA of `paired_ratio` × `(1.0 - avg_delay_normalized)`
- `self_caused_event_ratio`: paired events / total events
- `external_event_ratio`: unpaired consequence windows / total consequence windows

---

## 实现位置

| 变更 | 文件 |
|------|------|
| 新增自体字段 | `bionic_v0/src/self_development.rs` — 扩展 `SelfDevelopmentState` |
| 新增自体采集 | `bionic_v0/src/self_development.rs` — 扩展 `tick_self_development_from_substrate` |
| Snapshot 序列化 | `bionic_v0/src/snapshot.rs` — 已有 self_development 字段，沿用 |
| Observer 输出 | `bionic_v0/src/bin/observer.rs` — 已有 self_development 列，沿用 |
| Curriculum 证据 | `bionic_v0/src/curriculum.rs` — 新增 `SelfModelBoundaryEvidence` |
| 测试 | `bionic_v0/tests/self_development_life_loop.rs` — 扩展 |

不修改 `viewport.rs` 的 slot 分配器（self-model slot 池留空，待 v2+ 需要显式 slot 时再激活）。
不修改 `AGENTS.md`、`consciousness-stream-architecture.md`、`tier0-base-elements.md`。

---

## 里程碑

| M# | 名称 | 成功标准 |
|----|------|---------|
| S1 | **自体声音感知** | body 发声后 VIB return 的 delay+magnitude+similarity 三元组可区分 self vs external；`self_other_separation` 在有 vocal output 的 session 中 > 0 |
| S2 | **本体状态感知** | phot_fault 发生时 `body_state_awareness` 可观测上升；health 恢复后下降 |
| S3 | **行为-后果归属** | 高 agency 事件与 write epoch 在时间上对齐；external 事件与 write epoch 无关联 |
| S4 | **自體三層證據通過** | `self_model_boundary_gate_pass = 1`（三層均有 > 0 的數值證據且 source hygiene 通過） |

---

## 原则守护 Gates

| # | Gate | 检查方法 |
|----|------|---------|
| G1 | **无语义污染** | 新增代码无 `identity/self/ego/owner/personality/consciousness` 标识符（除 `self_development` 已有模块名） |
| G2 | **数值 substrate 完整** | observer 可输出所有新字段 |
| G3 | **可回溯验证** | 所有新字段均可从 snapshot 历史重新计算 |
| G4 | **不新增模块** | diff 确认只修改 `self_development.rs`，不新增文件 |
| G5 | **最高原则** | 无 body-side println；无 request/response；无 chat |

---

## 不在此 spec 范围内

- 自我模型的 slot 显式分配（v2+）
- 自我模型的长期记忆/叙事（v2+）
- 性格系统接入（v1+）
- 主人身份绑定
- 自体感与 probe law 的直接联动（v1+）
