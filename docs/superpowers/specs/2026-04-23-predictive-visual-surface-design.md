# Predictive Visual Surface Design

> **DEPRECATED / SUPERSEDED 2026-05-18**:
> This design is superseded for all new surface-expression work by
> [Organic Surface Expression Substrate v0](./2026-05-18-organic-surface-expression-substrate-design.md).
> It remains valid only as historical context for the first observer-side
> projection modules (`surface_projection`, `surface_history`,
> `surface_render`, `observer_surface`). Do not extend this document as the
> authority for organ-like pixel expression, regulated brightness, primitive
> reaction drives, or face-like / brain-activity external display behavior.

> **创建**：2026-04-23
> **状态**：v1.0 设计冻结
> **基于**：
> - [consciousness-stream-architecture.md](../../../consciousness-stream-architecture.md)
> - [tier0-base-elements.md](../../../tier0-base-elements.md)
> - [2026-04-20-new-axioms-v1.md](../../axioms/2026-04-20-new-axioms-v1.md)
> - [2026-04-20-bionic-v0-architecture-audit-cleanup-design.md](./2026-04-20-bionic-v0-architecture-audit-cleanup-design.md)
> **目的**：定义一个预测驱动、observer-side、非语言的视觉外显面，使 `bionic_v0` 的当前对象化预测与轨迹状态能够以原生图形方式被外部观看，而不把显示面误写成身体内部本体

---

## 1. 目的与边界

本 spec 定义的不是新的身体模块，而是一个新的 `外部观察渲染形态`：

- 它读取 `snapshot.bin`
- 它把当前对象化预测投影成实时图形
- 它不向 `body` 回写
- 它不在 `body` 内部生成人类语言
- 它不把图形面直接等同为身体内部本体

本 spec 只处理 `预测驱动视觉外显面` 的第一阶段设计，不处理：

- 新的本体层
- 新的命名层
- 新的语义/概念编号器
- 语言表达
- 情绪词汇系统
- 人格模块
- observer 与 body 的重新耦合
- 一体化启动器
- 身体内部“可主动控制显示”的正式输出通道实现

本 spec 只回答一个问题：

`在不污染 body 的前提下，如何把当前对象、轨迹、聚焦、预测稳定度等数值事实，投影为一种原生的、粗颗粒的、非语言的、实时视觉外显面。`

---

## 2. 定位

该外显面的正确位置是：

`body`
-> `snapshot.bin`
-> `observer-side visual surface`
-> `human researcher`

它不是：

- 身体内部器官的正式实现
- 语言输出
- 调试字符串面板
- 世界模型本体
- 对象命名器

第一阶段应被理解为：

`对象化预测的外部投影器`

而不是：

`AI 已经拥有的最终内部视觉母语器官`

这里必须固定两个边界：

1. `外显面` 是合法目标，但第一阶段仍然属于 observer-side 渲染，不得写回 body。
2. 用户描述中的“母语”“理解态”“想到什么就显示什么”在当前阶段都只能被视为 `外部研发目标描述`，不能被直接下沉为新的内部本体字段或命名层。

---

## 3. 高风险术语边界说明

根据 cleanup spec §8.3，本设计中出现的高风险描述必须先固边界。

### 3.1 “理解态”

- 这里是 `外部描述`
- 不是内部 ontology unit
- 不是新建的结构体或字段名
- 在第一阶段里，它只指 observer-side 从对象、空间、轨迹、预测误差、聚焦份额等现有数值上所看到的整体投影效果

### 3.2 “母语”

- 这里是 `外部比喻`
- 不是身体内部语言本体
- 不意味着重新引入词、标签、概念编号
- 只表示：系统未来可能拥有一种非文字、非人类语言优先的外显方式

### 3.3 “情绪 / 性格 / 身份”

这些词不进入第一阶段设计：

- 不作为快照字段
- 不作为颜色字典
- 不作为渲染状态机
- 不作为 observer 命名解释

如果未来真的需要讨论这些词，必须另开 spec，并逐条说明它们为何只是外部描述或工程索引，而不是内部本体。

---

## 4. 目标现象学

第一阶段的目标不是“重建摄像头看到的画面”，而是让以下现象可见：

- 当前活跃对象的大致位置
- 当前对象的大致范围
- 对象最近的移动轨迹
- 当前聚焦正在偏向哪里
- 对象是趋于稳定、扩散、漂移还是抖动
- 多对象之间的靠近、分离或追逐趋势

因此，第一阶段画面的正确目标是：

- `粗颗粒`
- `连贯`
- `非文字`
- `实时`
- `对象中心化`

而不是：

- 精细轮廓
- 写实视觉重建
- HUD 面板
- 指标堆叠
- 人工情绪配色说明卡

---

## 5. 第一阶段架构

第一阶段采用 `observer-side 独立二进制` 方案。

建议新增以下结构：

- `src/bin/observer_surface.rs`
- `src/surface_projection.rs`
- `src/surface_history.rs`
- `src/surface_render.rs`

职责边界如下。

### 5.1 `observer_surface`

负责：

- 读取 `runtime_dir/snapshot.bin`
- 驱动窗口事件循环
- 调度投影与渲染

不负责：

- 修改 snapshot
- 修改 viewport
- 写回 body
- 解释对象语义名称

### 5.2 `surface_projection`

负责把 `SnapshotData` 投影成 `SurfaceFrame`：

- 哪些对象进入当前画面
- 每个对象的位置、范围、强度、趋势
- 当前需要保留的轨迹段

它只能读现有快照事实，不能虚构不存在的语义层。

### 5.3 `surface_history`

负责 observer-side 的短时历史：

- 拖尾
- 残影
- 淡出
- 平滑

这些历史只能存在于外部观察器进程中，不能反向写回 body。

### 5.4 `surface_render`

负责将 `SurfaceFrame` 绘制到粗像素画布：

- 团块
- 轨迹
- 扰动
- 聚散
- 空间密度

渲染器不解释 meaning，只执行像素合成。

---

## 6. 输入合同

第一阶段只能读取 `snapshot.bin` 中已经存在的事实字段。

核心读取源如下：

### 6.1 对象槽骨架

来自 `SnapshotSlot`：

- `active`
- `last_update_t_us`
- `source_modality`
- `nomination_seq`
- `attention_share`
- `precision_bias`

这些字段决定：

- 对象是否存在
- 是否新鲜
- 当前是否更值得被显示

### 6.2 空间子树

来自 `SnapshotSpatial`：

- `pos_x`
- `pos_y`
- `span_x`
- `span_y`
- `precision`
- `valid`

这些字段决定：

- 对象中心
- 对象范围
- 空间外显的基本几何

### 6.3 预测与稳定度

来自：

- `slot.high_layer`
- `slot.phot / vib / mech` 的 `predicted / err / surprise`
- `slot.fusion`

这些字段决定：

- 图形更凝聚还是更扩散
- 外显更平稳还是更扰动
- 该对象更像一个稳定团块还是一个解离团块

### 6.4 全局选择信息

第一阶段可选读取：

- `global_attention`

其作用只能是辅助决定：

- 当前哪些对象应优先被看到

不得把全局 attention 渲染成调试面板中心。

---

## 7. 第一阶段能做什么，不能做什么

### 7.1 可以诚实做到的效果

- 对象团块
- 对象范围
- 对象轨迹
- 聚焦偏向
- 稳定/不稳定的图形差异
- 多对象之间的空间关系倾向

### 7.2 不能诚实声称做到的效果

- 精细视觉轮廓重建
- 对象类别识别外显
- “想到什么就显示什么”的高层自由表达
- 情绪颜色字典
- 人类可读 meaning 面板

第一阶段必须承认：

它显示的是 `对象化预测的外部投影`
而不是 `世界图像复原`

---

## 8. 视觉投影原则

第一阶段不应采用“固定颜色 = 固定人类语义”的设计。

也就是说，以下做法应被禁止：

- 红 = 愤怒
- 蓝 = 平静
- 橙 = 正常
- 白 = 理性

这些都属于设计者强加的解释，不是当前架构允许的原生外显。

第一阶段应遵守以下原则：

### 8.1 颜色不是语义字典

颜色只能是 observer-side 的数值投影结果：

- 可由多个数值场共同影响
- 可作为可替换投影策略
- 不得在 spec 中被提升为固定人类情绪词义

### 8.2 颗粒优先于细节

为了降低压迫感并保持高性能，第一阶段应采用：

- 粗像素
- 大颗粒
- 低频闪烁
- 缓慢衰减

### 8.3 屏幕空间是范围映射，不是说明板

屏幕上的空间分布应被理解为：

- 当前视觉范围
- 当前对象分布
- 当前聚焦偏向
- 当前对象化预测的外显压缩

它不应退化成：

- 文本指标面板
- 图例板
- 人类术语控制台

---

## 9. 渲染模型

第一阶段推荐使用以下 observer-side 合成模型。

### 9.1 对象团块层

每个活跃 slot 投影为一个粗粒对象团块。

其几何来源：

- 中心：`pos_x / pos_y`
- 范围：`span_x / span_y`
- 存活度：`active + last_update_t_us`

### 9.2 轨迹层

observer-side 维护每个 slot 的短时历史：

- 上一位置
- 过去若干帧的位置段
- 衰减权重

由此生成拖尾、残影、轨迹场。

### 9.3 聚焦层

使用 `attention_share` 与 `high_layer.attention_bias` 决定：

- 哪个对象更靠前
- 哪个对象更凝聚
- 哪个对象更新更积极

### 9.4 扰动层

使用 `predicted_surprise / object_err / family surprise` 决定：

- 图形边缘是否稳定
- 团块是否更碎裂
- 拖尾是否更震荡

注意：

扰动不是“情绪贴图”，而是纯数值外显。

---

## 10. 性能合同

第一阶段必须优先满足稳定和低压，而不是追求写实。

推荐实现策略：

- 内部低分辨率画布：例如 `160x90`、`240x135` 或同量级
- 整倍数放大到窗口
- 粗像素栅格
- 60 Hz 左右显示刷新
- 有上限的短时历史缓存
- 对同时显示的活跃对象数设置 soft cap

必须避免：

- 高分辨率逐像素重建
- 大量 CPU 粒子爆炸
- 高频闪烁
- 每帧重新分配大块内存

---

## 11. 禁令

第一阶段新增的视觉外显面必须遵守以下禁令：

1. 不得向 `body` 回写任何显示选择结果
2. 不得在 `body` 主循环中加入 GUI、文字输出或 observer 逻辑
3. 不得把颜色映射固定成人类情绪词典
4. 不得把显示字段命名为 `meaning / identity / emotion / personality / label / concept`
5. 不得把 observer-side 的短时轨迹缓存写回 snapshot
6. 不得把显示面误写为世界模型本体

---

## 12. 未来升级路径

本 spec 只冻结第一阶段，但必须给出不违例的升级方向。

### 12.1 阶段一：observer-side 投影器

当前 spec 所定义的阶段。

特点：

- 只读快照
- 不回写
- 外部渲染

### 12.2 阶段二：合法的外显参数通道

未来如果需要“系统自己决定显示什么”，必须新增一个 `纯数值的外显参数通道`。

该通道应满足：

- 不是语言
- 不是标签
- 不是概念编号
- 不是 observer 反向控制
- 只是一组可被身体调动的外显参数场

此阶段必须另开 spec，不得在本 spec 中偷渡实现。

### 12.3 阶段三：身体可调动外显面

届时外部窗口不再只是“从快照猜画面”，而是把该外显参数场渲染出来。

但这只能建立在阶段二合法落地之后。

---

## 13. 接受标准

第一阶段若要被视为达标，必须同时满足：

1. `body` 代码职责不扩大，仍然只负责持续运行与写快照
2. 图形窗口完全 observer-side
3. 画面能稳定显示活跃对象的大致位置、范围与轨迹
4. 画面不依赖文本标签
5. 画面不使用固定人类情绪语义字典
6. 在现有快照停止更新时，窗口行为应诚实退化，不制造虚假生命迹象

---

## 14. 结论

`Predictive Visual Surface` 的第一阶段，不是新的身体本体层，而是一个遵守最高原则的 observer-side 外显投影器。

它的职责很窄：

- 读取当前对象化预测
- 读取当前空间与轨迹事实
- 把这些事实投影成粗颗粒、实时、非语言图形

它不负责：

- 命名
- 解释
- 情绪字典
- 身体回写
- 世界本体定义

因此，第一阶段的正确完成标准不是“它看起来多聪明”，而是：

`它是否在不污染 body 的前提下，让对象化预测第一次以原生图形方式外显。`
