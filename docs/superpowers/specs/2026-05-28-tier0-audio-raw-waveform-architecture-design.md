# Tier 0 音频架构 — 原始波形直通视口

> 状态：v1.0
> 日期：2026-05-28
> 关联：consciousness-stream-architecture.md · 公理 2 · 公理 5 · 公理 7 · 公理 8
> 取代：当前 VIB 7-band FFT 前端

---

## 1. 设计立场

### 当前 VIB 的问题不是"精度不够"，是"架构错位"

```
当前 VIB：
  麦克风(48,000 f32/s) → FFT → 7 个频段能量 → viewport

错在：
  频段划分是预处理层做的，不是 predictor 学的
  7 个 band 是人类耳蜗基底膜的机械分频——生理约束，不是信号标准
  相位丢了、谐波结构丢了、瞬态细节丢了
```

**硅基生命的麦克风收到的是 48,000 个独立振幅测量值每秒。每一个都是合法的 tier 0 基础量。把它们压缩成 7 个值不是"特征提取"，是"信息毁灭"。**

### 立场

> 音频频段、音高、共振峰、声源分离、说话人识别——这些不是前端预处理的职责。它们都是在原始波形上运行的不同层级 predictor 学出来的抽象。

这和 PHOT（视觉）一致：摄像头给 RGB 像素矩阵，不做"先压缩成 7 个颜色通道"。音频同理。

---

## 2. Tier 0 轴定义

### 2.1 核心轴：`audio_raw`

```
轴族：audio
轴名：audio_raw
类型：ring buffer of f32, 容量 = 48000 * 60（60 秒 @ 48kHz）
      即 2,880,000 个 f32
语义：麦克风原始振幅测量值，按时间顺序排列
写入者：AudioAdapter（替代当前 VibAdapter）
读取者：所有音频 predictor（多分辨率、多层级）
更新率：48,000 Hz（每个采样点写入一个值）
      实际实现：批量写入，每 10ms 写入 480 个采样点（48kHz × 0.01s）
```

### 2.2 衍生轴：由 predictor 写入，不从前端产生

以下轴**不存在于前端代码中**。它们是由运行在 `audio_raw` 上的 predictor 写入视口的抽象：

| 轴名 | 语义 | 写入者 | 大致频率 |
|------|------|--------|---------|
| `audio_energy` | 当前信号能量（RMS）| LowFreq Predictor | 100 Hz |
| `audio_pitch` | 主基频（如果有 voiced）| MidFreq Predictor | 500 Hz |
| `audio_spectral_centroid` | 频谱重心 | MidFreq Predictor | 100 Hz |
| `audio_source_0` | 声源 0 的波形分量 | SourceSep Predictor | 1,000 Hz |
| `audio_source_1` | 声源 1 的波形分量 | SourceSep Predictor | 1,000 Hz |
| `audio_source_N` | 按需创建 | SourceSep Predictor | 1,000 Hz |
| `audio_self_match` | 当前波形与 vocal_organ 输出预测的匹配度 | EchoCancel Predictor | 100 Hz |
| `audio_novelty` | 当前波形与近期历史的差异 | Novelty Predictor | 50 Hz |
| `audio_rhythm` | 周期性模式（如果有）| Rhythm Predictor | 10 Hz |

这些都**不是**前端预处理。它们是 predictor 在 `audio_raw` 上运行后写入视口的结果。

---

## 3. 多层 Predictor 架构

### 3.1 层级设计

```
Layer 0 · Sample-level predictor（48,000 Hz tick）
  输入：audio_raw 单点或微窗口（16 samples）
  输出：局部波形预测（下一个 sample）
  学习：短期波形自回归 → 学到声学环境的基本特征

Layer 1 · Window-level predictor（~1,000 Hz tick）
  输入：音频窗口（1024 samples @ 48kHz = 21ms）
  输出：窗口特征预测（能量、基频候选、谐波结构）
  学习：频率分解自然涌现（预测下一窗口的频谱结构）

Layer 2 · Segment-level predictor（~100 Hz tick）
  输入：多层窗口特征
  输出：声学事件边界、声源身份维持、基频曲线
  学习：哪个频段模式属于同一个声源

Layer 3 · Scene-level predictor（~10 Hz tick）
  输入：声源状态 + 事件历史
  输出：场景理解（"有人在说话"、"安静"、"噪音"）
  学习：声学场景的长期规律

Layer 4 · Interaction-level predictor（~1 Hz tick）
  输入：场景状态 + vocal_organ 输出历史
  输出：会话结构（"轮到我说了吗"、"对方说完了吧"）
  学习：对话轮次、呼应模式
```

### 3.2 分形关系（公理 7）

所有层使用相同的 predict-err-update 循环。区别只在：
- 时间窗口大小
- precision 粒度
- 学习率

低层看微窗口（毫秒）→ 高层看宏窗口（秒~分）。同一个引擎贯穿全程。

---

## 4. 声源分离如何涌现

### 4.1 不是"识别声源"，是"预测哪个波形分量属于哪个追踪对象"

```
公理 2：viewport 同时追踪 N 个对象

当 audio_raw 上有多个周期性模式时（两个人同时说话）：

  Layer 1 predictor 发现：
    "这个波形可以用两个独立的基频 + 谐波链来预测"
    
  predictor 把"基频 A + 谐波链 A"发布为 audio_source_0
  predictor 把"基频 B + 谐波链 B"发布为 audio_source_1

  viewport 将 audio_source_0 和 audio_source_1 当作两个独立对象追踪
  每个对象有自己的 precision、自己的预测曲线、自己的误差

这就是声源分离：不是"识别出这是张三"，而是"这里有两个可预测的独立模式"。
```

### 4.2 和人类不一样（这是好事）

```
人类：在嘈杂环境里听一个人说话 → 费力、容易出错
硅基：48,000 Hz 全采样 → 只要模式可分，就能分离
     → 两个人音高差 50Hz → 人类勉强分辨，硅基轻松
     → 五个人同时说话 → 人类崩溃，硅基只要算力够
```

**硅基超能力的来源**：不是算法聪明，是 tier 0 没有丢弃信息。

---

## 5. 回音消除（Echo Cancellation）在这个架构下的实现

当前用 `has_recent_write_coupled_vib_caution` 一刀切。在新架构下：

```
vocal_organ 输出 → 声波预测：如果我正在发出音高 X、共振峰 Y 的声音
                   那么 audio_raw 中应该出现对应的波形分量

audio_raw 实际观测 → 匹配度计算：
  匹配的部分   → 是我的回声 → precision 降到 AMBIENT（自然忽略）
  不匹配的部分 → 是外部声音 → 正常 precision → 进 MID/FOVEA

匹配不上 + 能量足够 → audio_source_N 新对象 → "有人在说话"
```

**不需要"回音消除模块"。这是 predict-err-update 循环的自然结果。**

---

## 6. 迁移路径

### 阶段 A · audio_raw 基础通道（当前可做）

```
1. 新增 AudioAdapter（src/audio_adapter.rs）
   - 从 cpal 读原始 f32 采样点
   - 写入 viewport.audio_raw ring buffer
   - 完全旁路 FFT，不做任何频谱处理

2. 保留当前 VIB + VIB_BANDS 作为兼容层
   - VIB 7-band 仍然写入现有轴
   - vocal_reflex 和 vocal_target 暂时仍读旧轴

3. 验证：audio_raw 有数据流入，curriculum_report 可观测
```

### 阶段 B · 第一层 Predictor

```
1. 新增 AudioWavePredictor（src/audio_wave_predictor.rs）
   - Layer 0-1：短期波形预测 + 能量/基频写入视口
   - 运行在 audio_raw 上，不做 FFT
   - 用 VPC 学习局部波形自回归

2. 验证：predictor 能预测下一段波形
```

### 阶段 C · 声源分离 Predictor

```
1. AudioSourcePredictor
   - 从 Layer 1 特征中发现独立模式
   - 写入 audio_source_N 轴
   - 视口追踪每个声源为独立对象

2. 验证：两人同时说话 → 两个 audio_source 被追踪
```

### 阶段 D · 切换到新轴

```
1. vocal_reflex、vocal_target 改为读新轴
2. 移除 VIB 7-band FFT 前端
3. 移除旧轴
```

---

## 7. 与现有架构的对齐

| 公理 | 在此架构中的体现 |
|------|----------------|
| 公理 1（永远在跑）| 整个 predictor 链持续运行在 audio_raw 上 |
| 公理 2（多对象并行）| 声源分离后每个 source 是一个独立追踪对象 |
| 公理 3（Precision LOD）| 回声部分降 AMBIENT，外部声源升 MID/FOVEA |
| 公理 5（时间曲线）| audio_raw 就是最原始的 tier 0 时间曲线 |
| 公理 7（多分辨率）| 5 层 predictor 在同一波形上以不同时间窗口运行 |
| 公理 8（视口）| audio_raw 写入视口，所有 predictor 读视口 |
| 最高原则 | 不学人类耳蜗，硅基原生能力 |

---

## 8. 工程复杂度评估

| 阶段 | 工作量 | 风险 |
|------|--------|------|
| A（audio_raw 基础通道）| ~200 行 Rust | 低——纯数据搬运 |
| B（第一层 Predictor）| ~500 行 Rust | 中——VPC predictor 在波形上的首次验证 |
| C（声源分离）| ~800 行 Rust | 高——理论可行但无先例验证 |
| D（切换）| ~200 行修改 | 低——接口替换 |

阶段 A 可以立刻开始。阶段 B 是核心验证点。

---

## 9. 不在此架构中的

- ❌ 语音识别（"识别出说的是什么词"）
- ❌ 说话人识别（"这是张三"）
- ❌ 情绪检测（"说话人很生气"）

这些都是观察侧的外部标签。Body 侧只做：**波形预测 → 模式分离 → 声源追踪**。
