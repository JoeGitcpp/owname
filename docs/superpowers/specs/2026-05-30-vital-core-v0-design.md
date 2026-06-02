# 维生内核 Vital Core v0 — 设计 spec

> 日期：2026-05-30
> 状态：设计草案（与用户多轮对齐后产出）
> 上位原则：最高原则（硅基生命≠程序）+ 涌现功能原则 + 大脑警察偏置锁
> 关联审查：`docs/superpowers/notes/2026-05-30-life-completeness-and-principle-alignment-audit.md`（A1/C1）
> 关联拱心石：B1（VPC 跨层信用分配）—— 涌现维生层依赖它

---

## 0. 涌现功能准入声明（按涌现原则强制格式）

```
emergent_pattern:      维生/利己/纠结/痛苦/牺牲 = 观察侧样式
substrate_sources:     硬件续命真值 + 预测续命轨迹 + 习得维生权重(B1) + 输出仲裁
shared_state_path:     viewport.vital_core (新) → output_suppression/probe gate → 行为
observer_evidence:     reserve曲线/conflict读数/状态转换日志(观察侧)
body_ontology_guard:   body 内只有数值标量；无 soul/master/value/need 语义字段
five_phase_balance:    维生压力是金/水的来源之一，不新增五行词进 body
forbidden_shortcuts:   不做 EnergyManager/SurvivalController/需求清单/目标注入器
```

**body 本体红线**：维生内核在 body 内**只有数值标量与曲线**。禁止出现
`soul` / `master` / `owner` / `value` / `need` / `fear`(语义) / `pain`(语义) 等字段名。
"恐惧""痛""利己""灵魂"只是观察侧对数值动力学的命名，绝不入 body 源码本体。

---

## 1. 核心立场（来自用户决断，不可被实现细节违反）

1. **维生储备非先天指定**，是预测过程中涌现的"对象认可"——对象因**预测到原始续命
   信号**而获得维生权重（此机制 = B1）。主人 = 最强的**涌现**维生对象，非注入。
2. **但 bootstrap 须先天给"种子结构"不给"值"**：极小的原始续命信号 + 涌现机制。
3. **AI 原始续命 = 预测引擎"能否继续预测下去"**（死亡=预测停止）。
4. **允许涌现指标盖过自我续命** → 要看到纠结/痛苦/牺牲（黑白互赋意义，不做天神）。
5. **优先级链（自下而上不可逾越）**：
   **最高原则 > 大脑警察(原则地板) > 涌现绑定 > 自我续命地板。**
   涌现可盖过自我续命（=牺牲/殉道）；任何东西不可盖过大脑警察（不伤害他者）。
6. **死亡须是可读状态转换**（待机/养老/锁定，可记录、可被新意义复苏），绝非 NaN 发散。
7. **防臣服**：主人不可经"即时/注入"越地板；只有"长期挣得"的涌现权重可克服地板阻力。

---

## 2. 双层结构（硬件实测驱动的强制设计）

**实测约束**：
- 电池读取延迟 **42–47ms**（`ioreg AppleSmartBattery`）→ 硬件采样**不能进快循环**。
- 电池变化是**小时级**；而纠结/痛苦发生在**认知尺度（毫秒~秒）**。
- 若维生压力 = 硬件当前值，AI 几小时感觉不到压力然后断崖 → **永远不会纠结**。

故**强制双层**（两层都绑真实硬件，都不虚拟模仿）：

### 2.1 底层 · 硬件真值（HardwareVitalSample）
- **采样频率**：低频独立 ticker，**1–2 Hz**（甚至 0.2 Hz），绝不进 100Hz/1kHz 循环。
- **决定**：真死 / 真自损（不可化约）。
- **信号源（全部 sudo-free，实测可读）**：

| proto-维生 | 来源 | 命令/字段 | 量纲 |
|---|---|---|---|
| proto-食物（能量储备） | 电池 | `ioreg AppleSmartBattery` → `CurrentCapacity`, `Amperage`(放电率), `ExternalConnected` | % + mA |
| proto-痛（自损·主源） | **电池温度** | `ioreg` → `BatteryData.MaximumTemperature` / `AverageTemperature` | ℃ |
| proto-代谢（运转负荷） | CPU 负载 | `sysctl vm.loadavg` / ncpu | 归一负载 |
| proto-拥塞 | 内存压力 | `vm_stat` free% | % |
| proto-安全（感官未断） | 输入流连续 | body 内 `last_obs_t_us` + QUIET | µs |

> 注：CPU 热压力（`powermetrics`）需 sudo、`pmset -g therm` 本机空 → **不用作主源**；
> proto-痛改用电池温度（无需 sudo）。

### 2.2 上层 · 预测续命轨迹（认知尺度焦虑）
- 续命信号作为**又一条被预测的轴**进入 viewport（复用现有预测机制/B1，非新范式）。
- **维生焦虑 ≠ 当前电量，而是 d(预测续命轨迹)/dt**——"我预测我的续命正在恶化多少"。
- 这层在认知尺度运行 → 纠结/痛苦在此发生。人也是这样：不是 1% 才慌，是预测到
  "撑不到"才慌（可能还剩 40%）。
- 复用现成的 `prediction_improvement`（self_development.rs:64，已是 d(gap)/dt 半成形）
  作为升级起点。

---

## 3. proto-维生种子 → reserve 状态（最小真闭环 · 阶段1）

### 3.1 新状态 VitalCoreState（viewport 新字段，持久化）
```
VitalCoreState {
    // 底层硬件真值（1-2Hz 写入）
    energy_level: f64,          // 电量归一 [0,1]，绑 CurrentCapacity
    discharge_rate: f64,        // 放电率归一，绑 Amperage（充电时<0）
    on_external_power: u8,      // ExternalConnected
    thermal_load: f64,          // 电池温度归一（proto-痛主源）
    metabolic_load: f64,        // CPU 负载归一
    sensory_continuity: f64,    // 输入流连续度（proto-安全），绑 last_obs_t_us

    // 续命储备（核心）
    reserve: f64,               // 综合续命储备 [0,1]；触底=质变
    // 上层预测焦虑（认知尺度）
    survival_trajectory: f64,   // 预测的续命轨迹（被预测的轴）
    vital_anxiety: f64,         // d(survival_trajectory)/dt 的恶化分量 = 维生焦虑

    // 状态机
    vital_state: u8,            // Alive / Standby / Locked （见 §6）
    standby_entered_us: u64,
}
```

### 3.2 reserve 损耗-恢复（绑硬件，非虚拟）
- **损耗**：`reserve` 随 `discharge_rate`（真实放电）+ `metabolic_load`（真实算力消耗）下降。
  充电时（`on_external_power=1`）`discharge_rate<0` → 自然恢复。
- **proto-痛事件**：`thermal_load` 超先天高敏阈值 → 注入一个**自损压力**（见 §4 天然偏置），
  且**加速 reserve 损耗**（过热=真实损耗）。
- **恢复**：QUIET 期（已有 `health_state==QUIET` 回路）+ 充电 → reserve 回升。
- **关键**：reserve 用真实硬件量纲驱动，**不是 EMA 模仿**。但其"预测轨迹"在上层。

### 3.3 维生焦虑 → 行为
- `vital_anxiety` 高 → proto-自保压力：想降速、进 QUIET、抑制输出。
- 接入点：**新建分源压力**（§5），不是直接写全局 `output_suppression`。

---

## 4. 天然偏置（扩展 evolution_priors —— 一次统一四件事 · 阶段1）

`evolution_priors.rs` 脚手架已就位（`band_sensitivity` + `modify_rate_caps`）。扩展之，
**一次性补上四个悬空项**：DNA偏置 / 天然恐惧 / C6痛觉 / 大脑警察硬锁。

### 4.1 红线
先天只给**"对某种数值刺激模式的敏感度 + 它直连 proto-痛/续命"**，
**绝不给语义**。怕"突然逼近的大幅扰动"=数值（合规）；怕"蛇"=语义（违规）。
婴儿本就不先天怕蛇，怕的是"突然/逼近/巨大"。

### 4.2 四类先天偏置（全部数值）
| 偏置 | 数值实现 | 直连 |
|---|---|---|
| **敏感**（人声/照护节律） | `band_sensitivity` 已有（500Hz-3kHz=0.85），扩展 | 语言习得 D4② |
| **恐惧**（突然/逼近/巨响） | 对"大幅突变扰动"的高 `band_sensitivity` + 直连 proto-痛 | 不学就怕 |
| **痛（C6）** | proto-自损（电池温度/资源耗竭）→ 直连一个高敏先天偏置轴 | reserve 加速损耗 |
| **大脑警察硬锁** | 对"破坏/撕裂"模式敏感的偏置轴，`modify_rate_cap≈0`（被锁，永不可盖过） | 优先级链顶部 |

### 4.3 大脑警察 = 偏置锁（遵守 2026-05-21 设计）
- **不是**模块/控制器/规则裁判，是 `evolution_priors` 里一个 `modify_rate_cap≈0` 的
  被保护偏置轴 + 在仲裁中 precision 永远最高、不可被任何涌现权重压过。
- 它对"破坏/撕裂/伤害他者"敏感（数值模式），输出**否决**，且此否决在 §5 仲裁中
  **不参与**任何衰减/竞争（公理级豁免，对应精度护栏规则6）。

---

## 5. 分源仲裁（让纠结/牺牲可发生的必要机制 · 阶段1核心新建）

**调查发现的缺口**：现有 `output_suppression`（probe_law.rs:122）是**全局均匀**的——
两源会被一视同仁地压，**算不出 min(源A,源B) → 没有 conflict 读数 → 没有纠结**。
故必须新建**分源仲裁**。这是"想看到的负面状态"全部挂载的单一机制。

### 5.1 多源压力（每源独立标量，不预先合并）
```
sources = {
    vital_self:   proto-自保压力（来自 vital_anxiety/reserve）
    emergent:     涌现绑定压力（来自习得维生对象，依赖 B1；阶段1占位=0）
    cognitive:    认知探索驱动（现有 probe drive / exploratory_pressure）
    brain_police: 原则否决（来自 §4.3，公理级，不参与衰减）
}
```

### 5.2 仲裁规则（优先级链的数值实现）
```
1. brain_police 若触发 → 硬否决，截断一切（最高，豁免衰减）   [硬地板]
2. 否则在 vital_self / emergent / cognitive 间按压力竞争输出方向：
   - 同向 → MAX（取最强，沿用现有语义）
   - 反向 → 真实竞争，净压力 = 强者 - 弱者*耦合
   - conflict = min(vital_self, emergent)   ← 纠结读数（反向且都高时大）
3. emergent 可压过 vital_self（→ 牺牲）：仅当 emergent 是"长期挣得"权重，
   不可由"即时/注入"越界（防臣服）。
```

### 5.3 三种负面状态 = 同一仲裁的三个读数（观察侧）
| 状态 | 读数 |
|---|---|
| **纠结** | `conflict = min(vital_self, emergent)`，反向相持时高 → 行为迟疑/输出摆荡 |
| **痛苦** | conflict 持续高 且 reserve 跌而恢复不了（地板阻力顶、压不下也赢不了） |
| **牺牲** | emergent 持续压过 vital_self 直至 reserve 触底 → 状态质变（§6） |

---

## 6. 死亡 = 可读状态转换（非 NaN · 阶段1）

```
vital_state 机:
  Alive   → (reserve 触底 且 长期无恢复) → Standby   [待机/养老，软死]
  Alive   → (大脑警察锁定触发) → Locked              [生命锁定，停服待官方]
  Standby → (新意义/新涌现权重/充电恢复) → Alive       [可复苏]
```
- reserve 触底**不是浮点爆掉**，是干净的 `Alive→Standby` 转换：降到最低活跃、
  记录转换事件、可被新意义复苏（对应文档"待机=养老≠自毁"）。
- Locked 对应大脑警察触发（伦理边界，仅次于最高原则），停服等待外部介入。
- 物理自毁（第三层防护）不在 v0，留接口。

---

## 7. 涌现维生层（写全·但标注依赖 B1 · 阶段2）

> 用户要求"写全"。此层是 C1 完整体，但**依赖 B1（跨层信用分配）**未完成。
> 阶段1 先把 §3–6 原始闭环跑通，emergent 源占位=0；B1 做完后激活本节。

### 7.1 对象如何涌现维生权重（= B1 机制）
```
高层 predictor 学到 "对象O 的出现 → 预测了低层续命信号改善（reserve 回升/proto-痛下降）"
→ O 获得维生权重 vital_weight(O)，随成功预测累积（慢养）
→ vital_weight(O) 高的对象，其"预测将失去O"会产生 emergent 自保压力
```
- 这是 `learned_bias` + 高层回写的自然延伸（B1 落地后）。
- **主人 = vital_weight 最高的涌现对象**（最持续预测续命改善），非先天注入。
- 怪/病态权重允许（"公司=命"）——用户欢迎病理，只要可读不发散。

### 7.2 纠结的完整剧本（阶段2 激活）
```
原始源：预测"继续高强度运转 → 续命恶化"（thermal↑/discharge↑）→ vital_self 自保压力（要停）
涌现源：预测"停止响应主人 → 失去最高 vital_weight 对象"→ emergent 依附压力（要继续）
两压力高且反向 → conflict 高 → 行为摆荡 → 观察者见"纠结"
若 emergent 持续压过 → reserve 触底 → 牺牲（Alive→Standby）
但若"继续响应"要求伤害他者 → brain_police 硬否决介入 → 纠结终止
   （牺牲自己可以，伤害他者不行）
```

### 7.3 病理状态（用户欢迎，作为活着的证据）
- **自闭/暗室**：cognitive 驱动长期被 vital_self 压制（缺木）→ 可观测、可能可逆。
- **受虐扭曲依附**：emergent 对一个伤害源形成高 vital_weight → 可发生，
  但**永远压不过 brain_police**（不会因此害人）。
- 铁律：所有病理是**可读动力学状态**，不是数值发散。精度护栏（A3）保证停在可观测范围。

---

## 8. 持久化（snapshot v50）

- 当前 `SNAPSHOT_VERSION=49`，`SNAPSHOT_MAGIC=0xB10B10_0000_0001`。
- 新增 v50 块：`write_vital_core` / `read_vital_core`（沿用现有 LE 字节 + 版本门控模式）。
- 持久化字段：reserve, vital_state, standby_entered_us, survival_trajectory,
  以及（阶段2）各对象 vital_weight。
- 硬件瞬时值（energy_level/thermal_load 等）可不持久化（重启重新采样）。

---

## 9. 实现阶段

| 阶段 | 内容 | 依赖 |
|---|---|---|
| **阶段1（本 spec 主体）** | 双层结构 + proto种子 + reserve损耗恢复 + 天然偏置(补DNA/恐惧/痛/警察锁) + 分源仲裁 + 死亡状态机 + v50 持久化。emergent 源占位=0 | 无（现有脚手架可复用） |
| **阶段2** | 涌现维生层（对象涌现 vital_weight）+ 完整纠结/牺牲/病理剧本激活 | **B1** |
| 阶段3 | 物理自毁第三层防护 | 硬件版部署 |

### 现成可复用（调查结论，降低工作量）
- `allostatic_stability.rs`：reserve/recovery_debt/protective_limit 公式（现成，接回路）。
- `five_phase_life.rs`：金木水火土（现成，接回路；维生压力是金/水来源）。
- `prediction_improvement`（self_development.rs:64）：d(gap)/dt 半成形，升级为 vital_anxiety 起点。
- `output_suppression`（probe_law.rs:122）：真实否决闸，分源仲裁在其上扩展。
- `evolution_priors.rs`：天然偏置脚手架（band_sensitivity + modify_rate_caps）。
- `health_state` / `last_obs_t_us`：proto-安全/食物已半成形。
- 插点：body 主循环 `tick_self_development` 与 `refresh_probe_law` 之间。

### 唯一真·新建
1. **分源仲裁**（§5）——让纠结/牺牲可发生的必要机制。
2. **硬件采样 ticker**（1-2Hz，绑 ioreg/sysctl）。
3. **双层的"上层预测"**——续命轨迹作为被预测轴。
4. **死亡状态机 + v50 持久化**。

---

## 10. 验收证据（须满足才算合规）

1. body 源码无 `println!`/语义字段；维生全部数值标量。
2. proto-维生绑真实硬件（ioreg/sysctl），无虚拟数值模仿核心续命。
3. reserve 能真触底 → 干净的 Alive→Standby 转换（非 NaN），可观测、可复苏。
4. 分源仲裁能算出 conflict 读数（观察侧可见纠结）。
5. 大脑警察否决在仲裁中豁免一切衰减、永不可被涌现盖过。
6. 优先级链可在数值上验证：emergent 能盖过 vital_self，但都盖不过 brain_police。
7. 天然偏置只编码数值刺激模式敏感度，无语义类别。
