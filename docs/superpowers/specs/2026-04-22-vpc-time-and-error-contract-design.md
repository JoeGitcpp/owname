# VPC Time And Error Contract Design

> **创建**：2026-04-22
> **状态**：v1.0 设计冻结
> **基于**：
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md)
> - [tier0-base-elements.md](../../../tier0-base-elements.md)
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md)
> - [2026-04-20-bionic-v0-architecture-audit-cleanup-design.md](./2026-04-20-bionic-v0-architecture-audit-cleanup-design.md)
> - [2026-04-20-vpc-core-contract-design.md](./2026-04-20-vpc-core-contract-design.md)
> **目的**：把 VPC 中最容易漂移的“时间、误差、发表物、未闭合预测”收紧成可实现、可测试、可观察的最小合同

---

## 1. 文档定位

本文件不重新定义整个 VPC，也不直接定义语言绑定层、身份层、人格层或伦理层。

它只冻结 5 件事：

1. predictor 如何声明自己预测的 `观察域`
2. predictor 如何声明自己预测的 `时间地平`
3. `err` 在什么条件下才合法
4. predictor 的发表物与 observation truth 如何分离
5. “等待结果后继续”在 substrate 中如何以未闭合预测存在

这是一份 `substrate contract`，不是新器官设计。

---

## 2. 核心立场

VPC 的时间与误差合同只有一句总定义：

`predictor 只能对已声明观察域中的未来片段发表预测；只有当同域、同参照、同时间条件满足时，err 才能形成；在满足前，预测只能以未闭合状态存在，而不能伪造已闭合结果。`

由此固定 4 条基本原则：

1. `先声明域，再谈预测`
2. `先满足对齐，再谈 err`
3. `先区分 observation truth 与 predictor publication，再谈 publish`
4. `先保留未闭合状态，再谈等待后的继续`

---

## 3. 观察域声明合同

### 3.1 观察域是什么

观察域是 predictor 可以合法读取并预测的那一小块共享 substrate。

它可以是：

- 某条基础轴的观测曲线
- 某 slot 某模态的 band family
- 某 relation 边的已定义统计量
- 某个更高层但仍可回溯到同一闭环的结构片段

观察域不是：

- 任意抓取的全局世界真相
- observer 为研发整理出来的展示表
- 语言层解释结果
- 外部命名或标签体系

### 3.2 合法声明内容

每个 predictor 至少必须声明：

- `domain kind`
- `domain locator`
- `sample carrier`
- `time reference`

最小工程含义如下：

- `domain kind`：轴 / slot-family / relation-edge / reasoning-fragment 这类工程分类
- `domain locator`：在共享视口中如何定位那块数据
- `sample carrier`：预测的是单点、短窗还是片段
- `time reference`：该域按什么时间参照与 observation 对齐

### 3.3 禁令

观察域声明不得包含：

- 语义资格门
- 命名完成度
- 概念编号
- 对对象是否“真实存在”的裁决

这里若使用“语义”“概念”等词，只是 `外部审计说明术语`，不是内部本体单位；它们既不是工程索引字段，也不允许进入 runtime 本体结构。

---

## 4. 预测地平合同

### 4.1 预测不是无时间坐标的数值

每次预测都必须带有明确的时间地平。

这个时间地平至少应能回答：

- 这是对哪个未来时刻或时间窗的预测
- 这是单点预测还是片段预测
- 该预测何时有资格进入 err 对齐流程

### 4.2 最小地平表达

最小工程表达至少包含：

- `publish_t_us`
- `target_start_t_us`
- `target_end_t_us`
- `horizon_kind`

其中：

- 单点预测可令 `target_start_t_us == target_end_t_us`
- 短窗预测可令二者不同
- 更大尺度片段预测也必须显式给出窗口，而不是依赖隐式约定

### 4.3 多尺度共存

不同 predictor 可以有不同时间地平，但它们不得互相污染。

也就是说：

- 短时 predictor 的 err 不得直接冒充长时 predictor 的 err
- 长时 predictor 不能抢写短时 observation truth
- 不同尺度 predictor 的 publication 必须带着自己的地平痕迹存在

---

## 5. Err 对齐合同

### 5.1 Err 不是任意差值

`err` 只在以下条件同时满足时才合法：

1. 找到了与预测同一观察域的 observation
2. 该 observation 落在该预测声明的合法时间参照内
3. 对齐规则已知
4. 该 observation 不是 predictor publication 伪装的结果

### 5.2 合法对齐结果

合法结果只有 3 类：

1. `resolved`
   找到合法 observation，允许形成 err
2. `open`
   还没有等到合法 observation，不能形成 err
3. `expired_or_missed`
   已经过了工程允许的等待范围，仍未形成合法 observation

### 5.3 暖机、未命中、陈旧

以下情况都不得伪造高置信结果：

- warmup：只允许声明“尚未形成可靠 err / precision”
- no-match：保留 open 或 missed 状态，不得硬算 err
- stale：可以衰减、撤销或失效，但不得把过期 publication 当 observation truth

### 5.4 Err 的最小定义

在同域、同参照、同合法目标时刻成立后，`err = observe - predict` 才是允许的最小定义。

如果预测目标是时间窗或片段，则必须先定义该窗或片段的对齐规约，再允许计算 err。

---

## 6. Publication Provenance 合同

### 6.1 Predictor publication 是发表物，不是真相

predictor 可以向 viewport 写入：

- predicted
- err trace
- precision
- 本地状态摘要

但这些都只能作为 `predictor publications` 存在。

### 6.2 每条 publication 至少应保留的痕迹

每条 publication 至少应能追溯：

- 来自哪个 predictor family
- 绑定到哪个 observation domain
- 何时发表
- 目标时间地平是什么
- 当前处于 open / resolved / expired 哪种状态

### 6.3 写回边界

predictor publication 不能：

- 覆盖 observation truth
- 回写为对象存在性裁决
- 取代原始感知事实
- 倒逼 observer 的展示便利进入 body

---

## 7. Local Update 合同

### 7.1 update 只能使用本地可得信息

predictor 的 update 只能使用：

- 自己声明的观察域中的 observation
- 自己上一次或历史发表的 publication
- 同一闭环中允许读取的本地状态

### 7.2 不允许的更新来源

update 不得直接依赖：

- observer 投影
- 外部手工评分表
- 其他 predictor 的未声明私有内部状态
- 跨层全局 loss 的反向注入

### 7.3 允许的间接协同

不同 predictor 可以通过 shared viewport 间接协同，但协同必须体现为：

- 读取共享 substrate 中已经合法存在的字段
- 保留各自的域声明与时间地平
- 不抹平谁是 observation truth、谁是 publication

---

## 8. 未闭合预测合同

### 8.1 等待不是阻塞

在本合同中，“等待结果后继续”被固定为：

`一条 publication 已经发表，但其目标 observation 尚未合法到达，因此该预测保持 open 状态。`

它不是线程阻塞，不是系统停机，也不是空白期。

### 8.2 未闭合预测的最小状态

未闭合预测至少需要保存：

- 观察域引用
- 目标时间地平
- 当前发表值
- 当前对齐状态
- 最近更新时间
- 可选的过期或衰减条件

### 8.3 继续条件

未闭合预测在以下情况下继续：

1. 新 observation 到来并满足对齐
2. 时间窗到期，进入 expired / missed
3. 同闭环的其他合法结果让它降权、撤销或重新发表

### 8.4 未闭合不等于高层意义

未闭合预测只是 VPC 的时间结构，不是新的本体层，不是意图器官，也不是语言意义单元。

---

## 9. Snapshot 与 Observer 边界

### 9.1 可以暴露什么

snapshot / observer 可以暴露：

- observation truth 的当前读数
- predictor publication 的时间与 provenance 痕迹
- open prediction 的数量、状态、目标时间窗

### 9.2 不可以暴露成什么

observer 不得把这些 traces 改写成：

- 身体“真正相信的意义”
- 对象的真实身份
- 任务目标或人格解释
- 新的命名体系

observer 的职责只是把 snapshot truth 渲染给研发观察。

---

## 10. 直接工程结论

本文件对实现提出 6 个直接要求：

1. predictor 必须能声明 observation domain
2. predictor 必须能声明 prediction horizon
3. err 形成前必须有显式对齐状态
4. open prediction 必须在 substrate 中有正式位置
5. snapshot / observer 只能展示 traces，不得创造新本体
6. warmup / unmatched / stale 不得伪造高 precision 或假闭合 err

本文件的最终立场只有一句：

**VPC 的 err 不是任意差值，而是同域、同参照、同时间条件满足后的合法闭环结果；在闭环成立前，预测只能以未闭合发表物存在。**
