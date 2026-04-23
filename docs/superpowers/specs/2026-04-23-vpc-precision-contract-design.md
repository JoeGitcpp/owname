# VPC Precision Contract Design

> **创建**：2026-04-23
> **状态**：v1.0 设计冻结
> **基于**：
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md)
> - [tier0-base-elements.md](../../../tier0-base-elements.md)
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md)
> - [2026-04-20-vpc-core-contract-design.md](./2026-04-20-vpc-core-contract-design.md)
> - [2026-04-22-vpc-time-and-error-contract-design.md](./2026-04-22-vpc-time-and-error-contract-design.md)
> - [2026-04-22-vpc-predictor-contract-design.md](./2026-04-22-vpc-predictor-contract-design.md)
> **目的**：把 VPC 中 `precision` 字段统一为 `surprise` 语义，规范量纲，修正 fusion 层的量纲混合 bug

---

## 1. 文档定位

本文件只处理 predictor family 的 surprise/precision 数值语义和量纲对齐。

它回答的是：

- predictor publication 中的 `surprise` 是什么、怎么算、范围是什么
- 不同 family 的 surprise 是否可以直接混合
- fusion 层如何正确使用 surprise
- 哪些字段保留 `precision` 名字、为什么

---

## 2. 核心立场

`surprise 是 predictor 对自身预测误差的持续度量。它高表示预测与实际偏差大，它低表示预测与实际吻合。值为数值，不是标签，不是语义等级。`

由此固定 4 条原则：

1. `所有 predictor family 的 surprise 都是 EMA(err²)，范围 [0, 1]`
2. `warmup 期间 surprise 必须为零，不能伪造`
3. `同一量纲的 surprise 才可以混合；fusion 只混合同一量纲的 surprise`
4. `surprise 是数值曲线，不是语义分类、不是置信度的反向别名`

---

## 3. Surprise 计算合同

### 3.1 定义

每类 predictor family 的 surprise 按以下公式计算：

```
surprise = EMA_alpha(err²)
         = alpha * err² + (1 - alpha) * previous_surprise
```

其中：
- `err` 是该 predictor 的合法 err（已对齐 observation truth，见时间与误差合同）
- `alpha` 是各 predictor 自己的平滑参数
- 结果必须 Clamp 到 `[0.0, 1.0]`

### 3.2 各 Family 的 alpha 参数

| Family | Alpha | 备注 |
|--------|-------|------|
| `PhotSlotPredictor`（per-band） | `PHOT_PRED_SURPRISE_ALPHA = 0.05` | 较慢平滑，抗噪声 |
| `RelationEdgePred`（per-edge RMS） | `REL_PRED_SURPRISE_ALPHA = 0.1` | MSE EMA，与之前一致 |
| `Viewport.PredictorState`（per-axis，供 Ar2 派生） | 由 HighLevel predictor 写入，改为 EMA(err²) | 见 §5 |

### 3.3 Warmup 禁令

所有 family 在 warmup 期间：
- `surprise` 必须为 `0.0`（或 `_SURPRISE_MIN` = `0.0`）
- 不得根据 seed 值推断 surprise
- warmup 结束后的第一个 surprise 由真实 err² 驱动

---

## 4. Fusion 混合合同

### 4.1 量纲统一后

VIB 和 PHOT 的 band-level surprise 现在都是 `[0, 1]` 量纲。

`fused_surprise` 按 freshness-weighted mean 计算：

```
fused_surprise = (w_vib * vib_surprise + w_phot * phot_surprise) / (w_vib + w_phot)
```

其中 `w_vib` 和 `w_phot` 是新鲜度权重（与之前一致）。

### 4.2 不再混入非 surprise 值

以下内容**不**进入 `fused_surprise`：
- `precision_bias`（attention 派生的调制建议值）
- `SpatialState.precision`（LOD 置信度，方向与 surprise 相反）
- 任何手工调试摘要或人工注入权重

---

## 5. HighLevel Predictor 变更

之前 HighLevel predictor 把 `(mse / REF_MSE).clamp(0.2, 8.0)` 写入 `PredictorState.precision`，作为 Ar2Predictor 的学习率放大器。

现在改为：
- 直接计算每 axis 的 `surprise = EMA(err²).clamp(0, 1)`
- 写入 `PredictorState.surprise`（原 `precision` 字段改名）
- Ar2Predictor 读取 `surprise` 作为学习率乘子：`eff_lr = base_lr * surprise`
- `surprise` 高 = 误差大 = 学习率应该提高——行为语义不变，但量纲统一为 `[0, 1]`

**注意**：`base_lr * surprise` 在 surprise 低时（预测准）学习率低，这表示"不需要大调整"。这是合理的。

---

## 6. 命名映射

本次重命名只影响 predictor-family 和 fusion 域：

| 旧名 | 新名 | 影响范围 |
|------|------|----------|
| `precision`（`PhotSlotPredictor` 内部） | `surprise` | src/phot_predictor.rs |
| `precision`（`PhotPrediction` 输出） | `surprise` | src/phot_predictor.rs |
| `precision`（`BandFamilyState`） | `surprise` | src/viewport.rs, src/snapshot.rs |
| `precision`（`PredictorState`） | `surprise` | src/viewport.rs, src/adapter.rs, src/snapshot.rs |
| `fused_precision`（`CrossModalFusionState`） | `fused_surprise` | src/viewport.rs, src/snapshot.rs, src/object_high_layer.rs |
| `relation_precision`（`RelationEdgePredictionState`） | `relation_surprise` | src/viewport.rs, tests/ |
| `predicted_precision`（`ObjectHighLayerState`） | `predicted_surprise` | src/object_high_layer.rs, src/snapshot.rs |
| `smoothed_precision`（`ObjectHighLayerPredictor`） | `smoothed_surprise` | src/object_high_layer.rs |
| `mean_band_precision()` | `mean_band_surprise()` | src/viewport.rs |
| `set_precision()` | `set_surprise()` | src/viewport.rs |
| `PHOT_PRED_PRECISION_ALPHA` | `PHOT_PRED_SURPRISE_ALPHA` | src/lib.rs |
| `PHOT_PRED_PRECISION_MIN` | `PHOT_PRED_SURPRISE_MIN` | src/lib.rs |
| `PHOT_PRED_PRECISION_MAX` | `PHOT_PRED_SURPRISE_MAX` | src/lib.rs |
| `REL_PRED_PRECISION_MAX` | `REL_PRED_SURPRISE_MAX` | src/lib.rs |

### 保留 `precision` 名字不动的字段

| 字段 | 原因 |
|------|------|
| `ObjectSlot.precision_bias` | attention_share 派生的调制建议值，不是 surprise |
| `ObjectSlotSnapshot.precision_bias` | 同上 |
| `SpatialState.precision` | LOD 置信度，越高越精确，方向与 surprise 相反 |
| `SpatialEvidence.precision` | 同上 |
| `ATTN_PRECISION_BIAS_MAX` | 调制常量 |

---

## 7. 量纲变更对行为的影响

| 之前 | 之后 |
|------|------|
| Phot predictor surprise `[0, 100]` | `[0, 1]` |
| Relation edge surprise `[0, 2]` | `[0, 1]` |
| HighLevel → Ar2 precision `[0.2, 8]` (比例尺) | `[0, 1]` (EMA err²) |
| Fusion 混合 `[0.2, 8]` 和 `[0, 100]` | Fusion 混合统一 `[0, 1]` |

**关键：** `base_lr * surprise` 现在最大只有 `2.0 * 1.0 = 2.0`，而之前是 `2.0 * 8.0 = 16.0`。Ar2Predictor 在 high-surprise 情况下的调整力度会变小。这是**有意为之**——之前 surprise 高（预测差）时反而给了超大学习率，可能导致振荡。现在更平滑。

---

## 8. Snapshot 格式

二进制格式保持相同（`f64` 值直接写入/读取），但字段语义已变。旧的 snapshot 文件中 `precision` 位置的值在新的代码中会被当作 `surprise` 读取——数值范围不同可能导致显示异常，但这不影响运行中的 body。

旧 snapshot 需要重新生成或丢弃。这是研究系统重构过程中的正常行为，不是 bug。

---

## 9. Hypgiene 规则

新增源文件检查项：

- `bionic_v0/src/phot_predictor.rs`、`bionic_v0/src/viewport.rs`、`bionic_v0/src/object_high_layer.rs` 中的 predictor-family precision 相关代码不应再包含 `precision` 这个词（除了 §6 中保留不动的字段）
- 测试文件中对 surprise 值的断言不应使用 >1.0 或 <0.0 的值
- Fusion 测试应验证 fused_surprise 在各 band surprise 一致时不偏向某一 modality
