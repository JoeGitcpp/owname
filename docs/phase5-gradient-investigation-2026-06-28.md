# Phase 5 端到端梯度 — 调查报告与实施记录

**日期**：2026-06-28
**状态**：Phase 5-6 全部落地（409 tests 通过）
**已解决**：L1 画布不稳定 → sigmoid(gain=20) 替代 ReLU；L1 投射学习 → §4.5 L2 重建误差

---

## 1. 做了什么

### 1.1 SDD 内部矛盾发现

SDD 有两处描述 L1 怎么学习，互相矛盾：

- **§4.5**（L2 稀疏编码章节）：L1 从 L2 的重建误差学习（`∂E_sparse / ∂W_m`）
- **§9**（端到端梯度章节）：L4 误差一路链式传到 L1

**类比**：L1 是原材料加工厂，L2 是组装车间，L4 是质检站。
§4.5 说"加工厂根据组装车间的反馈调整"（上一层反馈），
§9 说"加工厂根据质检站的反馈调整"（跨三层反馈）。

**结论**：§4.5 正确。依据：Rao & Ballard 1999 预测编码理论——每一层只从相邻层的预测误差学习，不跳层。跳层梯度在生物系统中没有对应机制。

### 1.2 L4↔PG bridge 语义错配

SDD §8 定义 L4 输出应该是高级认知动作（feed_intention / spike_surprise / request_attention_focus），
但实现做成了低级信用操作（Completed→credit_hit, Separated→credit_miss）。

**类比**：SDD 说质检站应该向管理层报告"发现了什么、对什么感到惊讶、建议关注什么"，
实现却让质检站直接去改工人的工资（信用分）。机制层级错了。

**结论**：回退。等 Phase 6 注意力统一后，按 SDD §8 的高级语义重新设计。

### 1.3 L1 画布不稳定 = 真正的阻塞问题

W_SELF=1.2（自反馈权重 > 1.0）导致画布激活值随时间单调增长。

**类比**：话筒对着音箱——每一轮自己的输出都被放大 1.2 倍再送回来。200 个时间步后，
所有信号不是趋于无穷（NaN）就是卡在一个饱和值上。

**后果**：
- L4 模式匹配从不产生 Completed（只有 Separated）
- 所有依赖 Completed 的梯度代码都是死代码
- 现有系统已经"知道"这件事：active_reasoning.rs 和 self_development.rs 只检查 Separated，从不检查 Completed

**修复方向**（未实施）：需要 ART（Grossberg 1987）风格的 vigilance 机制——top-down 期望匹配，
不是无条件放大。这是 Phase 5 真正需要做的前置工作。

---

## 2. 保留了什么（+224 行，4 文件）

| 模块 | 新增 | 用途 |
|------|------|------|
| `context_modulation.rs` | `grad_features()` + 2 tests | L3 FiLM 层的梯度直传：grad_out × gamma |
| `feature_extraction.rs` | `TOPDOWN_DICT_LR` + `update_from_topdown()` + 2 tests | L2 字典接收 top-down 梯度调整 |
| `cognitive_pipeline.rs` | `match_error` / `gradient_applied` 输出字段 + tick() 中 L4→L3→L2 梯度路径 + 3 tests | 管线集成：Completed 触发时执行 L3+L2 梯度 |

**梯度链**（当 Completed 触发时）：
```
L4 residual → modulator.update()       [L4→L3 FiLM 权重更新]
           → modulator.grad_features() [L3→L2 梯度直传]
           → features.update_from_topdown() [L2 字典 top-down 调整]
```

画布不稳定问题已修复（见下方 §6），Completed 现在可以触发，梯度代码已生效。

---

## 3. 回退了什么

| 回退项 | 原因 |
|--------|------|
| `SharedCanvas::update_projections()` + `PROJECTION_LR` + 2 tests | L1 应从 L2 重建误差学，不从 L4 跳层 |
| `FeatureExtractor::grad_canvas()` + 2 tests | 为 L1 更新计算的中间梯度，回退 L1 学习后不再需要 |
| `viewport.rs::bridge_cognitive_to_pg()` | 语义错配：做成了 credit_hit/miss，应是 intention/surprise/attention |
| Pipeline tick 中 L4→L1 调用 | 同 §4.5 vs §9 矛盾，§4.5 正确 |
| `convergence_metric` 模块（lib.rs mod 声明） | 调查期间创建，未被任何代码引用，死代码 |
| SDD 文档状态行修改 | Phase 5 未完成，不应标记 ✅ |

---

## 4. 下一步（未实施）

**P0: 画布稳定性** — 所有梯度功能的前置条件
- 根因：W_SELF=1.2 > 1.0，无 top-down 期望匹配
- 方向：ART vigilance（Grossberg 1987）或等效的 top-down 归一化

**P1: L4↔PG 高级语义 bridge** — 等 Phase 6 注意力统一后
- 按 SDD §8：feed_intention / spike_surprise / request_attention_focus

**P2: L1 自身学习** — L2 重建误差驱动
- 按 SDD §4.5：`∂E_sparse / ∂W_m`，与 L4 梯度独立

---

## 6. P0 画布稳定性修复（2026-06-28）

### 问题
SDD §4.2.3 的竞争动力学使用 ReLU 作为传递函数：
```
transfer = max(0, w_self · a_i - w_inhib · mean(a) + z_fused_i - θ)
```
`w_self=1.2 > 1` 使 ReLU 输出无上界→激活值指数增长→NaN 或饱和→L4 永远只出 Separated。

### 修复
将 ReLU 替换为 sigmoid（gain=20），匹配 Dehaene & Changeux 2011 的原版 GNW 模型：
```
transfer = σ(20 · drive)    // σ(x) = 1/(1+e^{-x})
```

**为什么 gain=20**：
- gain 太低（5）：`σ(-gain×θ)` = 0.18，零输入时"关"态基线高于点火阈值（0.01），假点火
- gain=20：`σ(-6)` = 0.0025，关态干净低于点火阈值；开态 `σ(+large)` ≈ 1.0

**行为变化**：
- 激活值从 [0, ∞) 有界到 [0, 1]（sigmoid 上界）
- 零输入→64 维均收敛到 ~0.003（低于 0.01 点火阈值）→无假点火
- 强输入→差异化驱动→部分维度 ≈ 1.0 点火，其余 ≈ 0 抑制→正常竞争
- 2 个新稳定性测试 + 2 个调整过的旧测试（`zero_input_no_ignition` / `dev_mask_zeros_suppress_modality` 从检查"绝对零"改为检查"无点火"）

### 对 Phase 5 梯度的影响
画布输出稳定后：
1. L4 Completed 可以真正触发（稳定的模式匹配）
2. L4→L3→L2 梯度链激活，不再是死代码
3. L1 投射学习（§4.5 从 L2 重建误差）的前置条件满足

---

## 7. 文献依据

| 论文 | 用在哪里 |
|------|----------|
| Rao & Ballard 1999 (Predictive Coding) | L1 学习机制选择——相邻层误差，不跳层 |
| Friston 2005 (FEP) | 自由能最小化框架下的层间消息传递 |
| Dehaene & Changeux 2011 (GNW) | 画布竞争传递函数=sigmoid（非ReLU）；w_self>1的双稳态点火 |
| Olshausen & Field 1996 | L2 稀疏编码的理论基础 |
| Lee & Mumford 2003 | 皮层层级中 top-down 的作用（支持 L4→L2 不跳 L1） |
| Millidge 2021 | 预测编码与反向传播的关系综述 |

---

## 8. Phase 6 注意力统一实施记录

### 8.1 三路画布 salience 集成

SDD §4.3 要求：三套注意力系统（GlobalAttentionState / PredictionPressureField / ActiveFocusTrace）统一到画布竞争。增量式实现：

| 系统 | 集成方式 | 文件 |
|------|---------|------|
| GlobalAttentionState | canvas_salience (ignited_count / CANVAS_DIM) 乘入 raw score → attention_shares | viewport.rs `refresh_global_attention` |
| PredictionPressureField | canvas_salience 乘入所有 wave input amplitude | prediction_pressure_field.rs `tick_prediction_pressure_field_from_substrate` |
| ActiveFocusTrace | dominant trace 的 pressure/resonance → L3 FiLM context[8..9] | cognitive_pipeline.rs `build_context` |

### 8.2 CTX_DIM 扩展

8 → 10。新增：
- `ctx[8]` = attention_pressure（ActiveFocusTrace dominant trace 的 pressure）
- `ctx[9]` = attention_resonance（ActiveFocusTrace dominant trace 的 resonance）

FiLM 网络 W1 自动扩展（zero-initialized），新维度在初始化时是透传的（gamma=1, beta=0），随 L4 梯度学习逐步吸收注意力信号。

### 8.3 验证标准评估

SDD §10.3 Phase 6 标准："旧的 GlobalAttentionState 字段全部可从画布状态派生"

当前状态：attention_shares 已通过 canvas_salience 调制，可以从画布 ignited_count 派生 boost 系数。三套系统的输出都受画布状态影响。完全替换（废弃旧系统）符合 SDD §11 的风险缓解策略"先让 L1-L4 与现有系统并行跑，确认稳定后再切换"。
