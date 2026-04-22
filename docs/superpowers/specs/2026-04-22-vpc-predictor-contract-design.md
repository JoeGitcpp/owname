# VPC Predictor Contract Design

> **创建**：2026-04-22
> **状态**：v1.0 设计冻结
> **基于**：
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md)
> - [tier0-base-elements.md](../../../tier0-base-elements.md)
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md)
> - [2026-04-20-vpc-core-contract-design.md](./2026-04-20-vpc-core-contract-design.md)
> - [2026-04-22-vpc-time-and-error-contract-design.md](./2026-04-22-vpc-time-and-error-contract-design.md)
> **目的**：冻结 predictor family 的最小共同合同，确保不同 predictor 的域声明、warmup、err、publication provenance 与 local update 不再各自漂移

---

## 1. 文档定位

本文件只处理 `Predictor 合同`，不重写整个 VPC，也不定义语言层、伦理层、身份层或人格层。

它回答的是：

- 什么东西才有资格被称为 predictor family
- predictor family 最少必须声明什么
- warmup / err / precision / publication 在 family 之间必须保持哪些一致边界
- 现有不同 family 如何接到同一套 VPC provenance substrate

本文件当前覆盖 3 类 predictor family：

1. `Ar2Predictor`
2. `PhotSlotPredictor`
3. `relation predictor`

本版明确不纳入：

- `HighLevelPredictor`
- `object_high_layer`
- attention / routing / precision bias 这类调制或派生结构

---

## 2. 核心立场

predictor family 的总定义如下：

`predictor family 是一组在已声明 observation domain 上做未来发表、在合法对齐后形成 err、并仅用本地可得信息更新自身的局部预测闭环。`

由此固定 5 条总原则：

1. `family 必须先声明自己的 observation domain 与 horizon`
2. `family 必须有明确定义的 warmup 语义`
3. `family 只能在合法对齐后形成 err`
4. `family 的 publication 必须带 provenance`
5. `family 的 update 必须保持局部性`

---

## 3. Predictor Family 的共同合同

### 3.1 必备声明

每个 predictor family 至少必须声明：

- `family code`
- `domain kind`
- `domain locator`
- `horizon kind`
- `alignment rule`
- `warmup rule`
- `publication fields`
- `local update inputs`

### 3.2 Family code

`family code` 只是纯工程索引，用来区分不同 predictor provenance。

它不是：

- 概念编号
- 世界本体标签
- 命名资格
- 语义类别

这里若使用“类别”“family”等词，只是工程分组，不构成 ontology layer。

### 3.3 Publication 共同边界

所有 predictor family 都只能发表：

- `predicted`
- `err`
- `precision`
- 必要的 timing / provenance / warmup 痕迹

它们不能发表：

- observation truth 的替代值
- 对对象或关系是否存在的裁决
- 语言解释
- observer 专用展示语义

### 3.4 Warmup 共同边界

所有 family 都必须显式回答：

- warmup 何时开始
- warmup 何时结束
- warmup 期间允许输出什么
- warmup 期间禁止伪造什么

共同禁令如下：

- warmup 不得伪造高 precision
- warmup 不得伪造已闭合 err
- warmup 不能因为输出存在而被当作“已经可靠”

### 3.5 Local update 共同边界

所有 family 的 update 只能使用：

- 已声明 domain 内的 observation truth
- 自己先前的 publication
- 同闭环内允许读取的本地数值状态

它们不能直接使用：

- observer 投影
- 手工调试摘要
- 未声明的其他 predictor 私有内部状态
- 外部标签、词或语义门

---

## 4. Ar2Predictor 合同

### 4.1 Family 定义

`Ar2Predictor` 是基础轴级 predictor family。

它的 observation domain 是：

- 单条 `axis` 上的 observation ring

它的 horizon 是：

- 单点未来时刻

它的 publication 是：

- 某 axis 在未来单点的 predicted
- 与之对齐后的 err
- 同轴的 precision / predictor state

### 4.2 Warmup

`Ar2Predictor` 的 warmup 不是固定“前 N 次输出全零”，而是：

- 在 observation 少于 2 个样本时，不具备最小预测条件
- 在已有 pending publication 但目标 observation 尚未到达时，保持 open

因此它的 warmup / open-state 语义必须分开：

- `insufficient_context`
- `open_publication`

### 4.3 Err 合同

`Ar2Predictor` 的 err 只在以下条件成立时合法：

- 同一 axis
- 目标时刻已达到或越过
- 观察到新的 observation

---

## 5. PhotSlotPredictor 合同

### 5.1 Family 定义

`PhotSlotPredictor` 是 slot-local、PHOT band family predictor。

它的 observation domain 是：

- 某 active slot 中的 `phot band family`

它的 horizon 是：

- slot-local 的下一步 band family 预测

### 5.2 Warmup

它已经有明确 warmup 语义：

- `n_updates < PHOT_PRED_WARMUP_MIN`
- err = 0
- precision = 0

这套行为必须被正式视为 family contract，而不是实现巧合。

### 5.3 必须补上的共同合同位点

当前 `PhotSlotPredictor` 还缺少与 `Ar2Predictor` 一致的公开 provenance：

- predictor family code
- slot-family domain locator
- publish horizon
- open / resolved timing trace

本文件要求把这些接到现有 `vpc_contract` substrate，而不是另起 observer 专用字段。

---

## 6. Relation Predictor 合同

### 6.1 Family 定义

relation predictor 是 directed relation edge predictor family。

它的 observation domain 是：

- `GlobalRelationState.edges[i]` 的四维局部统计量

它的 horizon 是：

- 下一 cycle 的 relation publication

### 6.2 Warmup

它已经有 family-specific warmup：

- `warmup < REL_PRED_WARMUP_MIN`
- `relation_err = 0`
- `relation_precision = 0`

这必须被收编进 predictor 合同，而不只是局部注释。

### 6.3 Alignment

relation predictor 的 err 不是单标量输入差值，而是多维 publication 对当前 relation observation 的对齐结果。

因此该 family 必须显式声明：

- domain kind = relation edge
- carrier = multi-field relation fragment
- alignment rule = same-edge, same-cycle publication alignment

---

## 7. 现有三类 Family 的共同抽象

这三类 family 虽然 domain 不同，但共享以下抽象：

- 都有 family code
- 都在共享 viewport 上工作
- 都必须有 warmup 语义
- 都必须把 err 当作合法对齐后的结果
- 都不能让 publication 回写 observation truth

因此实现上应新增的是：

- predictor contract metadata
- predictor contract tests

而不是：

- 新的 predictor manager 器官
- 统一调度神经中枢
- 新的语义层

---

## 8. 与现有 VPC Contract 的关系

时间与误差合同已经回答了：

- 什么是 domain
- 什么是 horizon
- 什么是 open / resolved

本文件进一步回答：

- predictor family 如何强制声明这些东西
- 不同 family 如何把各自 warmup / err 语义映射到同一套 substrate 结构

因此两者关系如下：

- [2026-04-22-vpc-time-and-error-contract-design.md](./2026-04-22-vpc-time-and-error-contract-design.md) 是 `时间与状态合同`
- 本文件是 `family 层面接入合同`

---

## 9. 直接工程结论

本文件对实现提出 6 条直接要求：

1. `Ar2Predictor`、`PhotSlotPredictor`、relation predictor 都必须接入统一的 family provenance
2. 三类 family 都必须有可测试的 warmup 合同
3. 三类 family 都必须把 err 形成条件暴露成可测试行为
4. `PhotSlotPredictor` 与 relation predictor 不能继续只在本地结构里“隐式满足”合同
5. `vpc_contract` 应扩展为能容纳 family-level predictor traces，而不是每类各造一套
6. 任何新增字段都必须保持 numeric / engineering trace 性质，不得引入语义本体

本文件的最终立场只有一句：

**Predictor family 的差异可以保留在 domain 与 carrier 上，但 warmup、err 合法性、publication provenance 与 local update 边界必须收口到同一套 VPC 合同。**

