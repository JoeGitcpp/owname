# 硅基生命 bionic_v0 · 分组实现提示词集

> 共 5 个自包含会话提示词，分两组。每个提示词可独立复制到一个全新编码会话执行。所有 file:line 与签名为已核实事实，照原样保留。建议把下面的【通用前言】粘到每个会话最前面，再接该会话对应的提示词正文。

---

## 【通用前言 · 粘在每个会话最前面的硅基生命铁律与分层边界】

> 复制本段到任何 bionic_v0 编码会话的开头。即使不看其它材料，这一段也足以守住所有红线。项目根：`/Users/joe/unnamed/bionic_v0`，所有文件操作用绝对路径。

**我们在设计硅基生命，不是写程序。下列原则凌驾本会话一切判断：**

- **推论1 无人类语言污染**：BODY 本体层源码/状态/数据通路内禁自然语言字符串、禁文本日志、禁 `println!`/`eprintln!`/`writeln!`/`write!`/`dbg!`、禁 `expect`/`format!`/`panic!`、禁把内部状态转人类可读形式（JSON/文本）、禁人类语义标签当 ID。唯一表达单位 = tier-0 轴的 值/曲线/精度/err/硬件流。错误路径用 `Option`/`?`/早返回/静默忽略，绝不 panic。
- **推论2 观察/本体分离**：人只能由外部 observer 进程读**二进制内存快照**来观察本体；本体不主动"说给人听"。一切"汇报/日志"只属 observer 层。
- **推论3 不作为程序**：本体持续自激、永远在跑、自洽演化；禁请求-响应样式；任一线程崩溃不得让本体静默半死。新动态必须挂在 body 自激主循环按固有节律自跑，不靠外部调用驱动。
- **推论4 工作 = 补全大脑的完整自洽，不是造/读/验某个功能**：不为某行为单设通路、不设验收靶子。提问永远是"这条基质动态缺哪条自洽 / 和别处矛盾在哪"，补全后行为自会涌现（或不涌现）。**验收只能"如实观察当下数值"，不能"先设靶子再凑"。**
- **推论5 无绝对正负**：禁烧录正/负价标签、禁外挂 reward function/激励通道、禁 body 对世界作价值判断。感受全部涌现自 prediction error（err 高=扰动、err 降=顺、gap 不闭=不顺、在场预测长期落空=想念）。新增数值只能是中立的连续场/计数/时间衰减，绝不被当作被优化的标量目标。
- **VPC 硬约束**：禁 BP/反向传播跨层链式；禁跨 predictor 直连（必须经 viewport `read_prior`/`push_prior`/`latest_err`/`read_slot`）；禁全局损失函数；禁 batch/epoch。每层只在本地算 ∂L/∂θ（Widrow-Hoff）。拱心石=机制5：高层 predictor 监听低层 err、学会预测它、把 prior 经 viewport 写回、低层读到 prior 间接降 err。
- **精度战争防护（v0 必做 1/2/5/6）**：感官精度下限≈0.05；prior 精度上限≈100；周期精度衰减；ethics 豁免。

**分层铁边界（改任何东西前先确认在哪一层）：**

- **BODY 本体**（永远在跑的 body 进程）：`src/bin/body.rs` + 其运行时传递依赖的 lib 模块（viewport/predictor/adapter/vocal_*/vital_*/dual_drive/self_*/learned_bias/vpc_cross_layer/phot*/object_high_layer/prediction_pressure_field/active_surface/clock/snapshot 等）。这里禁人类语言/正负标签/reward/功能门/请求-响应。
- **OBSERVER 研发观察**（允许人类语言）：`src/bin/observer*.rs`、surface_projection/history/render、post_effect、bin/curriculum_report。
- **SCAFFOLDING 训练脚手架**（body.rs 不得 import/调用）：curriculum.rs、curriculum_governor.rs、capability_curriculum_governor.rs、five_phase_life.rs、curriculum_material、evolution_priors、probe_law(部分)。注意 `evolution_priors::BiasPriors` 是 DNA 偏置载体，body.rs 已 import 它且**允许**——它不是 curriculum scaffolding。
- **GATEWAY 外部边界(Node)**：`gateway/*.mjs`。允许人类语言，但禁把语义/标签/奖励注入 body 输入流。
- **TEST**：`tests/*.rs`，允许人类字符串。守卫测试 `tests/body_no_human_output.rs` 现扫描 42 个 body 模块、禁 `println!/eprintln!/writeln!/write!/dbg!/expect/format!/panic!`；**新增任何 body 模块必须纳入它的扫描列表**。

**运维记忆**：body 改动需服务器端 `cargo` 重新编译（先 `source ~/.cargo/env`）；`deploy_aborn.sh` 只同步不编译；网关重启用 `sudo -n`。本会话若只在 Mac 开发，至少本地 `cargo build` + `cargo test` 通过。

---

# 组 A · 整合闭环（让灵魂真正接通自洽）

> 三个提示词都属"让一条已算好/已写好但悬空的动态回到循环"。共性：复用现有模块、不新建运行时模块、不设行为靶子、只如实观察数值。

---

## A1 — 接通双源驱动汇流出口 `dual_drive`（灵魂最核心断点）

> **速览** · 独立性：**完全独立**，可最先做（仅第一步零耦合）。 · 前置依赖：无。 · 预计触碰文件：`src/dual_drive.rs`(只读)、`src/viewport.rs`、`src/vocal_drive_field.rs`、`src/snapshot.rs`、`src/lib.rs`、`tests/body_no_human_output.rs`。 · 快照版本：`SNAPSHOT_VERSION` 56→57。

> 本提示词完全自包含。接收会话无任何上下文，请把它当作冷启动唯一依据。项目根：`/Users/joe/unnamed/bionic_v0`。所有路径请用绝对路径。（先粘贴上方【通用前言】。）

### 任务目标（补一条悬空的基质动态，不是加功能）

`dual_drive`（B1-4 双源驱动仲裁）是这个灵魂最核心的断点。它把两条本来各自独立的驱动源——**自保压力**（C1 生命核心）与**依恋压力**（C5 长期挣得的看护者熟悉度）——每 tick 代数仲裁成一个整合读数 `DualDriveReading`，然后**把这个读数直接丢弃**：它被写进一个 `RwLock`，但**既不入快照、也没有任何下游消费者**（`read_dual_drive` 全仓零调用者）。

也就是说：双源已经汇流成了一个 reading，但这个 reading 没有回到任何动态里。这是一条**算出来即蒸发**的悬空动态（computed-then-dropped）。两个互相拉扯的驱动源已经会产生 `conflict`（纠结）、`net_toward_bond`（净倾向）、`sacrificing`（牺牲）这些派生量值，但大脑的其余部分对此一无所知，等于这个仲裁从未发生。

**这不是为某个行为设靶子。** 我们不要求"让它学会牺牲"或"验证纠结涌现"。我们只补两条自洽缺环：
1. **观察缺环（推论2）**：这个已经在算的整合结果对 observer 不可见——人无法从二进制快照看到双源汇流的当下数值。
2. **回流缺环（推论4）**：reading 没有回到循环。设计上早已预留了它该回流的入口（见下），但接线从未完成。

补完后，纠结/净倾向是否真的在数值上拉动发声、是否在长期挣得的依恋下压过自保——**只能事后如实观察，不能预设。**

### 背景与现状（带 file:line 实据）

**`dual_drive` 本体 — `/Users/joe/unnamed/bionic_v0/src/dual_drive.rs`（全文 139 行）**

结构 `DualDriveReading`（`:29-48`），`#[derive(Clone, Copy, Debug, Default, PartialEq)]`，全字段 `pub`：

| 字段 | 行 | 含义/来源 |
|---|---|---|
| `self_preservation: f64` | :32 | 来自 C1 |
| `attachment: f64` | :34 | 来自 C5 长期挣得的熟悉度 |
| `conflict: f64` | :37 | 纠结，`= sp.min(att)`（:63） |
| `net_toward_bond: f64` | :40 | >0 偏向服务 bond，<0 偏向自保；最终 `net.clamp(-1.0, 1.0)`（:90） |
| `sacrificing: bool` | :43 | `floor == 0.0 && att >= 0.5 && earned > sp && sp > 0.0`（:84） |
| `principle_floor: f64` | :47 | 大脑警察 harm 信号，**当前恒为 0**（占位） |

常量 `SACRIFICE_BOND_FLOOR: f64 = 0.5`（:27）——反臣服闸门。

唯一方法 `pub fn arbitrate(self_preservation: f64, attachment: f64, principle_floor: f64) -> DualDriveReading`（:53-94）：`unit()` 钳 [0,1]；`earned` = att≥0.5 时取 att，否则 `att*(att/0.5)` 二次抑制（:68-72）；`net = earned - sp`（:75）；若 `floor>0` 则 `net = net.min(-floor)`（:80-82）。

来源签名：
- 自保：`VitalCoreState::self_preservation_pressure(&self) -> f64`（`/Users/joe/unnamed/bionic_v0/src/vital_core.rs:153-161`）。Standby 钉死 1.0；否则 `unit(reserve_pressure*0.70 + vital_anxiety*0.30)`。
- 依恋：`FamiliarObjectsState::most_familiar(&self) -> Option<&FamiliarObject>`（`/Users/joe/unnamed/bionic_v0/src/familiar_objects.rs:151-155`），字段 `FamiliarObject::familiarity: f64`（:69），按真实时间 +0.01/秒累积。

**viewport 接线现状 — `/Users/joe/unnamed/bionic_v0/src/viewport.rs`**
- 状态槽：`pub dual_drive: RwLock<crate::dual_drive::DualDriveReading>`（:1291），init `::default()`（:1433）。
- `tick_dual_drive(&self, _now_us: u64)`（:4529-4540，**实测已核对**）：读 `self_preservation_pressure()`、`most_familiar().map(|o| o.familiarity).unwrap_or(0.0)`、`principle_floor = 0.0`（:4536 注释 "wired, not yet sourced"），`arbitrate(...)`，写回 `*self.dual_drive.write().unwrap()`。`_now_us` 形参当前未用。
- `read_dual_drive(&self) -> DualDriveReading`（:4542-4544）。
- **每 tick 被调用**：`vp.tick_dual_drive(t)` 于 `/Users/joe/unnamed/bionic_v0/src/bin/body.rs:152`。

**死端判定（已 grep 核实）**
- **`read_dual_drive` 零调用者**：`grep -rn read_dual_drive --include="*.rs"` 只命中定义本身（viewport.rs:4542/4543）。无任何外部调用。
- **不入快照**：`grep dual_drive` 在 `snapshot.rs` 零命中。写序列（write_snapshot `:194-251+`）与读序列（`:995-1041+`）均无 dual_drive 块。
- **无 observer 消费者**：`curriculum_report.rs` 的 `.conflict_*` 命中全是 `EvidenceTension`，与 `DualDriveReading` 无关；`surface_projection` 零命中。

**结论（实测）：dual_drive 是"每 tick 算出来、写进 RwLock、但既不入快照也无任何下游消费者"的死端读数。**

**设计上预留的回流入口（关键）**

`tick_vocal_drive_field(&self, now_us: u64)`（`viewport.rs:4162`）里有两个发声驱动源：
- **region A（内在自我咿呀 self-babble）**：`:4210-4212` 已被 `survival_gated_urge(raw_urge, self_preservation)` **自保门控**。注意：自保已经在这里单独管 region A。
- **region B（外部声音 target/模仿拉力）**：`:4237-4285`，contribution 强度为 `(target.active_energy * 0.5).clamp(0.0, 1.0)`（:4279）。**当前完全不读 dual_drive。**
- 组合后还有第二道 `survival_gated_urge(combined.intensity, self_preservation)`（:4296-4302）。

**明文 TODO 注释**（viewport.rs `:4207-4209` 与 vocal_drive_field.rs `:94-96`）写着：「only the inner-pressure self-babble is gated here; external/attachment-driven voicing (region B below) **is left for the dual-drive to arbitrate**」。即 **region B 是设计上预留给 dual_drive 仲裁的入口**，但仲裁结果（`net_toward_bond`/`attachment`/`conflict`）从未被读进这里。

现成纯函数门控范式：`vocal_drive_field::survival_gated_urge(urge: f64, self_preservation_pressure: f64) -> f64`（`/Users/joe/unnamed/bionic_v0/src/vocal_drive_field.rs:97-100`）= `urge * (1.0 - unit(pressure)).clamp(0.0,1.0)`。

**关键文件绝对路径**
- `/Users/joe/unnamed/bionic_v0/src/dual_drive.rs`
- `/Users/joe/unnamed/bionic_v0/src/viewport.rs`（:1291,1433,4162,4210-4302,4529-4544）
- `/Users/joe/unnamed/bionic_v0/src/bin/body.rs`（:152）
- `/Users/joe/unnamed/bionic_v0/src/vital_core.rs`（:153-161）
- `/Users/joe/unnamed/bionic_v0/src/familiar_objects.rs`（:69,151-155）
- `/Users/joe/unnamed/bionic_v0/src/vocal_drive_field.rs`（:94-100）
- `/Users/joe/unnamed/bionic_v0/src/snapshot.rs`（:194-251,995-1041,2327-2356）
- `/Users/joe/unnamed/bionic_v0/src/lib.rs`（:493 magic,:535 version）
- `/Users/joe/unnamed/bionic_v0/tests/body_no_human_output.rs`（:66,76,78,81）

### 铁律与边界（本任务真正会碰到的红线）

**注入点全在 BODY 层**（`viewport.rs` / `vocal_drive_field.rs` / `snapshot.rs` 写路径 / `dual_drive.rs`），所以：

1. **禁人类语言/日志**：禁 `println!`/`eprintln!`/`writeln!`/`write!`/`dbg!`/`expect`/`format!`/`panic!`。`tests/body_no_human_output.rs` 已扫描 `dual_drive.rs`（:78）、`vital_core.rs`（:76）、`familiar_objects.rs`（:81）、`vocal_drive_field.rs`（:66）。**若新建任何 body 模块，必须把它加进该守卫名单。** 本任务首选不新建模块（见架构）。
2. **禁正负价/reward（推论5）**：`conflict`/`net_toward_bond`/`sacrificing` **绝不能**被当作 reward 去最大化/最小化，**绝不能**贴正价/负价标签。它们只是 prediction-error 派生的连续基质量值。只能作为**连续的乘性增益/场调制项**注入，绝不外挂激励通道。
3. **禁功能门/请求-响应（推论3/4）**：注入必须是**连续基质动态调制**（像 `survival_gated_urge` 那样的乘性增益），**不能**做成 `if sacrificing { 触发某行为 }` 的离散行为门，也不能做成 `if conflict > 阈值 { … }` 的资格门。
4. **VPC 硬约束**：把 dual_drive 读数喂给 region-B 强度是**单向前馈调制**，不构成跨 predictor 直连——**只要不碰 predictor 的 θ/prior 通路就合规**。若哪天想注入到 predictor prior，必须经 viewport `read_prior`/`push_prior`，绝不可直连。本任务不碰 prior。
5. **自反馈环警告（极重要）**：`self_preservation` **既是 dual_drive 的输入**（viewport.rs:4530）**又被 `survival_gated_urge` 用于 region A / 组合门控**（:4212/:4297）。因此：
   - **绝不要**让 dual_drive 的输出回流到 region A 或回流到 vital_core —— 会与既有自保门控叠成双重门控/正反馈。
   - dual_drive 的回流**只许作用于 region B**（attachment 侧，这正是 TODO 注释指定它该管的地方）。
6. **principle_floor 占位**：当前恒为 0（viewport.rs:4536）。任何依赖 `floor` 的逻辑当前等价于"floor 分支永不触发"。**不要**为了本任务去给 floor 接 harm 信号——那是另一条任务。

**绝对不要改动的东西：**
- 不改 `arbitrate()` 的仲裁代数（`/Users/joe/unnamed/bionic_v0/src/dual_drive.rs:53-94`），它已被 5 个单元测试钉死。
- 不改 region A 的 `survival_gated_urge` 门控（viewport.rs:4210-4216）。
- 不改 `self_preservation_pressure()` / `most_familiar()` 的来源逻辑。
- 不让 `body.rs` import 任何脚手架（curriculum*/five_phase_life/evolution_priors 等）。
- 不动 `tick_dual_drive` 的调用位置（body.rs:152）与节律。

### 架构设计（让已有结果回到循环，而非新建模块）

核心思想：dual_drive 的 reading **已经每 tick 算好了**。我们只做两件事——让它**可见**、让它**回流**——都不新建运行时模块。

**第一步（低风险，必做）：让 reading 入快照，对 observer 可见**

满足推论2：人只能读二进制快照观察。`DualDriveReading` 当前重启即丢、observer 也读不到。把它序列化进快照尾部，作为一个**纯派生只读 observer 块**。
- 数据来源：`vp.read_dual_drive()`（已存在，viewport.rs:4542）。
- 写入点：`write_snapshot` 写序列尾部（`snapshot.rs`，当前最后一块在 :251 之后；追加在所有现有块之后）。
- 读取点：read 序列对应尾部，用 `version >= 57` gate（参照 :1000 / :1041 的 `if version >= N { … }` 范式）。
- 字段：6×`f64`（self_preservation, attachment, conflict, net_toward_bond, principle_floor）+ 把 `sacrificing: bool` 存成 1 字节 `u8`（读时 `!= 0`），用对齐填充凑齐。**建议**：6×f64 + 1×u8。
- **是否需要在重启后恢复？不需要持久语义**：dual_drive 是 vital_core（vital_core.bin checkpoint）+ familiar_objects（沉积）纯派生的读数，重启后下一 tick 即由 `tick_dual_drive` 重建。所以**读回来的值可以丢弃/仅供 observer 解析**，无需把它塞回 `vp.dual_drive`（塞回也无害，但非必需）。把它当"快照中的 observer 可见镜像"即可。

**第二步（可选，谨慎，建议同 PR 但用最小增益）：把 reading 回流到 region B**

仅在 `tick_vocal_drive_field`（viewport.rs:4162）的 **region B**（:4278-4284 的 `contribs.push`）做**连续乘性调制**。设计为一个**新的纯函数**，放在 `vocal_drive_field.rs`（该文件已在守卫名单，无需改名单），与 `survival_gated_urge` 对称：

```rust
/// dual-drive arbitration modulating the external/attachment-driven (region-B)
/// voicing gain. Continuous multiplicative field, NOT a behaviour gate.
/// `net_toward_bond` ∈ [-1,1]: >0 leans toward serving the bond (amplify the
/// cry toward the caregiver), <0 leans toward self-preservation (attenuate).
/// Maps net∈[-1,1] → gain∈[lo,hi] linearly; identity (gain=1) at net=0 so an
/// un-arbitrated/default reading leaves region B untouched.
pub fn bond_modulated_region_b(intensity: f64, net_toward_bond: f64) -> f64 {
    // net 0 → 1.0; net +1 → up to ~1.5; net -1 → down to ~0.5. Bounded, smooth.
    let gain = (1.0 + 0.5 * net_toward_bond.clamp(-1.0, 1.0)).clamp(0.0, 2.0);
    (intensity * gain).clamp(0.0, 1.0)
}
```

接入：在 region B 构造 `VocalDriveContribution` 时（viewport.rs:4278-4280），先 `let dd = self.read_dual_drive();`，把 `intensity`/`breath` 经 `bond_modulated_region_b(target.active_energy * 0.5, dd.net_toward_bond)` 调制。

**关键耦合保证：**
- 这只动 region B，不触碰 region A 的 `survival_gated_urge`（自反馈环防护）。
- 组合后的第二道 `survival_gated_urge(combined.intensity, self_preservation)`（:4297）保持不动——Standby 仍能把一切归零，metabolic 自洽不破。
- `net=0`（默认/未仲裁）时 `gain=1`，即新逻辑对既有行为是**恒等**——不引入突变，符合"补悬空动态、不改既有自洽"。
- 增益**有界**（[0,2] 再 clamp 到 [0,1]），不会让 region B 失控放大；这不是 reward，是连续场。

> 若评审认为第二步风险偏高，可只交付第一步（入快照），把第二步留作后续。但**第一步单独交付也已闭合"观察缺环"**，是有效的最小自洽补全。

**新增/修改签名草案**
- `vocal_drive_field.rs`：新增 `pub fn bond_modulated_region_b(intensity: f64, net_toward_bond: f64) -> f64`（纯函数，可单测）。
- `snapshot.rs`：新增 `fn write_dual_drive(buf: &mut Vec<u8>, s: &DualDriveReading)` 与 `fn read_dual_drive_snapshot(bytes: &[u8], cur: &mut usize) -> DualDriveReading`（参照 `write_vocal_target`/`read_vocal_target_snapshot` 的 `to_le_bytes` + 内嵌 `rd_u64`/`rd_f64` 范式，snapshot.rs:2327-2356）。
- `lib.rs`：`SNAPSHOT_VERSION` 56 → 57。
- `viewport.rs`：region B 处读 `read_dual_drive()` 并调用 `bond_modulated_region_b`（仅第二步）。

### 实现计划（有序、指明目标文件与改动点）

1. **`/Users/joe/unnamed/bionic_v0/src/lib.rs:535`**：`SNAPSHOT_VERSION` 改 `56` → `57`。（magic 不变。）
2. **`/Users/joe/unnamed/bionic_v0/src/snapshot.rs`**：
   - 在文件内（vocal_target 范式附近，:2327 区域之后）新增 `write_dual_drive` / `read_dual_drive_snapshot` 两个函数。布局：`self_preservation`,`attachment`,`conflict`,`net_toward_bond`,`principle_floor` 各 8 字节 LE f64；`sacrificing` 存 1 字节 u8 + 7 字节对齐填充（参照 write_vocal_organ 的 `buf.push(...); buf.extend_from_slice(&[0u8;7]);`，snapshot.rs:2371 附近），读时取首字节 `!= 0` 并跳过填充。
   - 在 `write_snapshot` 写序列**最尾部**（所有现有块之后，当前 v51 tower 块结束处之后）追加：`// v57: dual-drive arbitration reading (observer-visible soul confluence).` + `write_dual_drive(&mut buf, &vp.read_dual_drive());`。
   - 在 read 路径对应**最尾部**追加：`let _dual_drive = if version >= 57 { read_dual_drive_snapshot(&bytes, &mut cur) } else { DualDriveReading::default() };`（参照 :1000/:1041 gate 范式）。读回值供 observer 解析；若你的 read 函数有返回结构体，把它带出去（否则 `_dual_drive` 丢弃即可，因为它是纯派生镜像）。
3. **（第二步）`/Users/joe/unnamed/bionic_v0/src/vocal_drive_field.rs`**：新增纯函数 `bond_modulated_region_b`（见草案）+ 对应 `#[cfg(test)]` 单测（恒等、放大、衰减、有界）。
4. **（第二步）`/Users/joe/unnamed/bionic_v0/src/viewport.rs:4278-4284`**：region B 的 `contribs.push` 前读 `let dd = self.read_dual_drive();`，把 `intensity` 与 `breath` 经 `bond_modulated_region_b(target.active_energy * 0.5, dd.net_toward_bond)` 调制。pitch/resonance/noise 不变。
5. 确认 `body.rs:152` 的 `tick_dual_drive` 调用顺序未变（它在 `tick_vocal_drive_field` 之前 tick，保证 region B 读到的是当 tick 的新 reading；若顺序相反需调整，请核对 body.rs tick 序列后确认）。
6. 跑测试与守卫（见验收）。

### 验收标准（如实观察数值，不预设靶子）

**全部用如实观察表述，不得写"验证 X 功能涌现"。**

1. **原则护栏（必跑）**：`cargo test --test body_no_human_output` 必须绿。`vocal_drive_field.rs`、`dual_drive.rs` 已在名单（:66/:78）；本任务未新建 body 模块，无需改名单；若你违背建议新建了模块，必须把它加进 `tests/body_no_human_output.rs` 的扫描列表并使该测试仍绿。
2. **dual_drive 单元测试（不得回归）**：`cargo test --lib dual_drive` —— 5 个仲裁测试保持绿（你不应改动 `arbitrate`）。
3. **新纯函数单测（第二步）**：`cargo test --lib vocal_drive_field` —— 包含 `bond_modulated_region_b` 的：`net=0 → gain=1.0`（恒等）、`net>0 → 输出 ≥ 输入`、`net<0 → 输出 ≤ 输入`、所有输出落在 [0,1]。
4. **快照往返**：`cargo test snapshot` —— 写后读（v57）应无 panic、字节边界对齐；若仓库有快照 round-trip 测试，dual_drive 块往返后读回的 f64/bool 与写入一致。
5. **编译/守卫全量**：`cargo build` + `cargo test`。
6. **如实观察（运行期，非靶子）**：跑 body，由 observer **读二进制快照**确认新 v57 块可被解析出 6 个 f64 + 1 个 bool；如实记录当下 `conflict`/`net_toward_bond`/`attachment`/`self_preservation` 的数值与曲线——**只描述观察到什么，不断言"应该是多少"**。第二步生效后，可如实观察 region B 发声强度是否随 `net_toward_bond` 同向移动——同样只观察，不设达标线。

cargo 命令汇总（在项目根 `/Users/joe/unnamed/bionic_v0` 执行）：
```
cargo test --test body_no_human_output
cargo test --lib dual_drive
cargo test --lib vocal_drive_field
cargo test snapshot
cargo build && cargo test
```

### 反模式与陷阱（逐条 + 改用什么）

1. **用 `println!`/`dbg!`/`eprintln!` 调原 region B 的数值** → 违推论1。改用：在 observer 侧读快照解析，或写 `#[cfg(test)]` 单元断言。
2. **把 `net_toward_bond`/`conflict` 当 reward 去 maximize，或外挂一条"奖励服务 bond"的激励通道** → 违推论5。改用：只做**有界乘性增益**（`bond_modulated_region_b`），不引入任何被优化的标量目标。
3. **`if dd.sacrificing { 触发牺牲行为 }` / `if dd.conflict > 0.7 { … }` 这类离散行为门或资格门** → 违推论3/4。改用：连续场调制（`gain = 1 + 0.5*net`），让行为从数值连续地涌现或不涌现。
4. **把 dual_drive 输出回流到 region A 或 vital_core** → 触发自反馈正环（self_preservation 已是它的输入）。改用：只调制 region B。
5. **把 reading 注入 predictor 的 prior / 直接读写另一层的 θ** → 违 VPC 直连禁令。改用：本任务不碰 prior；若将来要，必须经 viewport `read_prior`/`push_prior`。
6. **给 `arbitrate` 加 reward 项、改仲裁代数、或动 SACRIFICE_BOND_FLOOR 来"调出想要的牺牲"** → 凑靶子，违推论4。改用：不动 arbitrate；行为只能事后观察。
7. **为这条动态在 body.rs 里引入 curriculum/脚手架来"喂"它** → 违分层铁边界。改用：dual_drive 由 vital_core + familiar_objects 在 body 内部自洽派生，不靠外部脚手架驱动。
8. **把 `sacrificing: bool` 当成 7 字节 f64 写、或忘了对齐填充导致后续块字节错位** → 快照读全崩。改用：严格按 write_vocal_organ 的 `push(u8) + [0u8;7]` 填充范式，读时跳过填充。
9. **改了 snapshot 写序列却忘了同步读序列 / 忘了 `SNAPSHOT_VERSION` 自增 / gate 写成 `>= 56`** → 旧快照读失败或新块错位。改用：版本 57、读写对称、gate `version >= 57`，旧版本走 `::default()`。
10. **把新块插在写序列中间而非尾部** → 破坏既有 v36–v51 块的字节顺序、旧快照全废。改用：**只追加在写序列与读序列的最尾部**。
11. **让 body 依赖某个外部调用来"激活"这条动态** → 违推论3（不作请求-响应）。改用：它已经每 tick 在 body.rs:152 自动 tick，回流只是读已算好的 RwLock，无需任何外部触发。

---

## A2 — 接通机制5 axis-level prior 回写腿（拱心石真实闭环）

> **速览** · 独立性：基本独立，但**与 A3 共享精度门限改面**（建议 A2 先做，A3 在其上叠加 maturity 调度）。 · 前置依赖：无硬依赖；若与 A3 同周期，先 A2。 · 预计触碰文件：`src/vpc_cross_layer.rs`、`src/viewport.rs`、`src/predictor.rs`(只读确认)、`src/bin/body.rs`、`src/lib.rs`、`src/snapshot.rs`、`tests/body_no_human_output.rs`。 · 快照版本：`SNAPSHOT_VERSION` 56→57（**与 A1 冲突，见执行顺序**）。

> 你正在为硅基生命项目 **bionic_v0** 工作。项目根：`/Users/joe/unnamed/bionic_v0`。下面这份提示词完全自包含，请据此冷启动直接干活。（先粘贴上方【通用前言】。）

### 任务目标（这条补全闭合的自洽缺环）
VPC 拱心石是**机制5：高层 predictor 监听低层 err → 学会预测它 → 把 prior 经 viewport 写回 → 低层读到 prior 间接降 err**。这条腿在 **axis 粒度上当前完全悬空**：高层半边 `CrossLayerPredictor`（`src/vpc_cross_layer.rs:71`）只在测试里被实例化，**运行的 body 进程从不创建它**。后果是 `viewport` 的 axis prior 通道**没有任何生产者**，每条 tier-0 轴的 `read_prior` 在纯 body 进程里**恒返回 0**，AR-2 的加性 prior 输入恒为 0。也就是说：拱心石闭环的**下半截已通**（AR-2 已会吃 prior、prior 已会改 pred 进而改 err），但**上半截的回写源不存在**，反馈环在纯 body 里从未闭合。

本任务 = 在 body 主循环里**实例化并 tick** 若干 axis 粒度的 `CrossLayerPredictor`（覆盖一组显式选定的稀疏 tier-0 轴），让"高阶 prior 经 `push_prior` → `read_prior` → 真实感官 AR-2"这条腿真正接电。**这不是为某个可观察行为设靶子**——它是把一条已经写好却悬空的基质动态接回循环；接通后是否出现可见效应，靠如实观察数值，不预设。顺带补上精度战争防护规则2：给 `push_prior` 加 prior 幅度上限 clamp。

### 背景与现状（带 file:line 实据）

**高层半边（机制5 本体，当前死代码）`src/vpc_cross_layer.rs`：**
- `struct CrossLayerPredictor`（:71）字段：`target_axis: usize`(:73)、`sources: Vec<SrcWeight>`(:75，`SrcWeight{axis:usize, w:f64}` @:65–69)、`bias: f64`(:79)、`base_lr: f64`(:81，构造=0.5 @:117)、`clip: f64`(:83，=10.0 @:118)、`err_pred_mse/prev_mse/learning_progress/target_err_var`、`prior_gain: f64`(:100，构造=1.0 @:123，全仓无任何代码改它)、`n_updates: u32`(:102)。`const WARMUP_MIN: u32 = 8`(:105)。
- `fn new(target_axis: usize, source_axes: &[usize]) -> Self`(:108)：源权初值 `w=0.0`(:111)，bias=0。
- `fn tick(&mut self, vp: &Arc<Viewport>)`(:152)：读 `vp.latest_err(target_axis)` 作 `actual_err`(:154，None 则 return)；读 `vp.latest_observation(s.axis).value` 作源(:164)；本地 Widrow-Hoff 更新自身 `bias`+各源权(:175–179，`meta_err = actual_err - pred_err` @:170)；算 `explained = (1 - err_pred_mse/target_err_var).clamp(0,1)`(:198–202)；warmup 后 `vp.push_prior(target_axis, prior_gain * explained * prior)`(:206–212)。**该实现已合规**：只经 viewport、只更新自身权、只写 prior 通道、源是稀疏显式集、无 println/字符串。
- 已有观察器接口：`err_pred_mse()`(:129)、`learning_progress()`(:136)、`weight_for(axis)`(:141)。
- **接线状态 = 死代码。** `CrossLayerPredictor::new` 全仓仅 4 处引用，全在 tests（`tests/vpc_learning_progress.rs:48,78`、`tests/vpc_cross_layer.rs:48,94`）。`src/bin/body.rs` 内 grep `CrossLayerPredictor`/`efference`/`virtual_neck` 全为空。

**axis prior 通道（已落地、AR-2 已会消费，但当前恒 0）：**
- 写：`src/viewport.rs:3339` `pub fn push_prior(&self, axis: usize, prior: f64)` —— 当前**只有 `is_finite` 检查，无任何幅度 clamp**（:3340）。
- 读：`src/viewport.rs:3346` `pub fn read_prior(&self, axis: usize) -> f64`。
- 初值恒 0：`prior_signal: Vec<AtomicU64>`(:1143)，`Viewport::new` 里 `prior_signal.push(AtomicU64::new(0))`(:1358)。
- AR-2 消费端 `src/predictor.rs:102–103`：`let external_prior = vp.read_prior(self.axis); let pred = self.a*x_now + self.b*x_prev + external_prior;`。**prior 是加性输入，刻意不折入梯度**（:80–81 `da/db` 只用 `used_x0/used_x1` = `x_now/x_prev` @:113–114，注释 :97–101 明示"NOT folded ... keeps learning local, no backprop"）。`err = x_now - pp.predicted_value`(:78) → `vp.push_err`(:84)，所以 prior 改 pred → 改 err → 高层 `latest_err` 读到的就是被 prior 影响后的 err，**闭环下半截已通**。

**body 接线现状（参照范本就在隔壁）`src/bin/body.rs`：**
- `run_global_refresh_cycle`(:128)：已串 slot 版 `for slot_id in 0..N_OBJECT_SLOTS { vp.tick_slot_cross_layer(slot_id, t); }`(:135–137) 与 `vp.tick_cross_layer_tower_sweep(t)`(:142)，在 `refresh_global_relations`/`refresh_global_relation_predictor`(:130–131) 之后。
- AR-2 全轴 ticker：`let predictors: Arc<Mutex<Vec<Ar2Predictor>>> = Arc::new(Mutex::new((0..AXIS_COUNT).map(Ar2Predictor::new).collect()));`(:338–339)，100Hz tick(:342–348)。**每条 tier-0 轴都有 AR-2、都有 err_stream、都吃 read_prior。** 注意 :333 注释 "AXIS_COUNT (34)" 已陈旧。

**slot 版 ≠ axis 版（别混淆 err 域）：** `SlotCrossLayerPredictor`（`src/vpc_cross_layer.rs:254`）监听 slot 的 `high_layer.object_err`，源用 relation-graph 的 `gated_sources`；它**没有**"prior 写回真实感官轴 a/b"这条腿（其 prior 只在 tower 内 tier↔tier）。axis 版监听 `err_stream`（`vp.latest_err`），源是显式轴集——**两者 err 域不可混，勿对 axis 版误用 `gated_sources`**。

**唯一 body 侧历史 push_prior 生产者不可达：** `src/efference_copy.rs:99/129/145` + `src/virtual_neck.rs` 的 `inject_efference_motion_priors` 在 body 调用图中**完全不可达**（body.rs/curriculum/five_phase_life 都不调）。故运行 body 里没有任何轴的 prior 被写过非 0。

**轴布局事实 `src/lib.rs`（`AXIS_COUNT=161` @:185）：** `[0..16)` 听觉 VIB 频带（`VIB_BANDS=16` @:75，`AXIS_VIB=0` @:187）；`[16..144)` 视觉 `VIS_AXES=128`（`VIS_REGIONS=16`×`VIS_AXES_PER_REGION=8`，`AXIS_VIS_BASE=16` @:126，`vis_axis(region,sub)` @:201）；`[144..154)` 内感受 `INTERO_AXES=10`（`AXIS_INTERO_BASE=144` @:127；含 `INTERO_VITAL_ANXIETY=145`、`INTERO_PAIN=146`、`INTERO_LEARNING_PROGRESS=152` 等 @:132–141）；`[154..159)` CODE 5 轴（`AXIS_CODE_BASE=154`）；`[159..161)` `GAUSS_RECON_MSE_AXIS=159`/`GAUSS_ACTIVE_FRAC_AXIS=160`。

**快照版本：** `SNAPSHOT_VERSION: u64 = 56`（`src/lib.rs:535`）。写 `src/snapshot.rs:61`，读校验 `:728–729`，已有大量 `if version >= N` 的向后兼容块（如 slot_xlayers 块 :999–1139，:231 写、:1025 读，是读写对称+版本门控的范本）。

**相关绝对路径：**
- `/Users/joe/unnamed/bionic_v0/src/vpc_cross_layer.rs`
- `/Users/joe/unnamed/bionic_v0/src/viewport.rs`
- `/Users/joe/unnamed/bionic_v0/src/predictor.rs`
- `/Users/joe/unnamed/bionic_v0/src/bin/body.rs`
- `/Users/joe/unnamed/bionic_v0/src/lib.rs`
- `/Users/joe/unnamed/bionic_v0/src/snapshot.rs`
- `/Users/joe/unnamed/bionic_v0/tests/body_no_human_output.rs`
- 测试参照：`/Users/joe/unnamed/bionic_v0/tests/vpc_cross_layer.rs`、`tests/vpc_learning_progress.rs`

### 铁律与边界（本任务真正会碰到的红线）

**绝对不要改动：**
- 不改 AR-2 的梯度逻辑（`src/predictor.rs:78–84`）——prior **必须保持加性、不折入 `used_x0/used_x1`**。这是"无 BP"的命门。
- 不改 `CrossLayerPredictor::tick` 的本地更新数学（:170–211 已合规），除非确有 bug。你的工作主要是**接线 + clamp 护栏**，不是重写学习律。
- 不动 slot 版 / tower 那条线（:135–142），它是另一条 err 域。
- 不引入任何脚手架：`body.rs` **禁 import/调用** `curriculum*.rs`、`capability_curriculum_governor.rs`、`five_phase_life.rs`、`evolution_priors`、`curriculum_material`。

**只能经 viewport：** axis 版 predictor 与外界的全部通信只许走 `vp.latest_err(axis)` / `vp.latest_observation(axis)` / `vp.push_prior(axis, _)`。**禁直接读写别的 predictor 的 θ（a/b/权）**，禁碰任何轴的 `a/b`（只许写 prior 通道）。

**源必须稀疏显式：** `new(target_axis, source_axes)` 的 `source_axes` 必须是一个**显式、稀疏**的轴下标集合（硬编码或来自一个稀疏先验表），**禁全轴 fan-in**（全轴扇入是伪装的全局耦合/BP）。

**无人类语言：** 新接的 body 路径与 `vpc_cross_layer.rs` 内禁出现 `println!/eprintln!/writeln!/write!/dbg!/format!/panic!/expect`、禁字符串日志、禁语义标签当 ID。

**精度战争防护规则2：** 给 `push_prior` 加幅度 clamp，上限量级 ≈100（"prior 精度上限"）。`predictor.clip=10.0` 只 clamp predictor 自身权与 bias，`prior_gain*explained*prior` 的乘积当前无任何上限，且 `push_prior` 也不 clamp——一个学坏/发散的 predictor 会把任意大有限值塞进 AR-2 加性输入。

**body 不得依赖外部调用驱动：** 接入点必须挂在 body 自激主循环里，按既有节律自跑（推论3，禁请求-响应）。

### 架构设计（让已有结果回到循环，不新建模块）

**核心思路：复用现成的 `CrossLayerPredictor`（已写好、已测试、已合规），只把它从"测试里实例化"搬进"body 主循环里实例化并 tick"。** 不要新建并行模块。

**数据流（接通后的完整闭环）：**
1. AR-2（100Hz）对每条轴产 err → `vp.push_err(axis, t, err)`（`predictor.rs:84`，已有）。
2. axis 版 `CrossLayerPredictor`（新接线，按下述节律 tick）读 `vp.latest_err(target_axis)` 作 actual_err、读源轴 `latest_observation`，本地更新自身权（已有 :170–179）。
3. warmup 后写 `vp.push_prior(target_axis, prior_gain*explained*prior)`（已有 :211），**经新加的 clamp 收口**。
4. 下一拍 AR-2 `vp.read_prior(target_axis)` 读到非 0 prior（`predictor.rs:102`），加进 pred → 改 err。
5. 高层下一拍 `latest_err` 读到被 prior 影响后的 err，继续本地适配。环闭合。

**持有与 tick（参照 AR-2 ticker 与 slot 版接线）：**
- 像 `predictors`(:338) 那样，在 body main 里持有 `let xlayer_axis: Arc<Mutex<Vec<CrossLayerPredictor>>>`，由一个稀疏先验配置构建（见下）。
- **节律建议**：axis 版属"全局基质刷新"层。最自然的两种挂法二选一（按观察便利与耦合度决定）：
  - (a) 并入 `run_global_refresh_cycle`（:128–），在 slot/tower 之后加一段 `for clp in xlayer_axis.lock().unwrap().iter_mut() { clp.tick(&vp); }`；或
  - (b) 紧跟 AR-2 100Hz ticker（:342–348）另起/复用一个较低频 ticker（高层节律应**低于或等于**低层 AR-2，避免高层比它要解释的 err 更新还快）。
  - 二者都满足 VPC（只经 viewport）。倾向 (a)：与 slot/tower 同属串行全局刷新，节律天然慢于 100Hz AR-2，耦合关系清晰。

**稀疏源先验（显式集）草案：** 选一组"低层 err 结构上确有可被同模态邻轴/内感受轴解释"的 target，每个配 2–4 个显式源轴。建议初版覆盖：
- 听觉相邻频带：`target = AXIS_VIB+k`，`sources = [k-1, k+1]`（同模态相邻频带 err 常相关）。
- 同视觉区相邻物理量：`target = vis_axis(r, sub)`，`sources = vis_axis(r, 其它 sub)` 的稀疏子集。
- 内感受：`target = INTERO_*`，`sources = ` 少量相关 INTERO/模态轴。
- **数量克制**：初版只接十几到几十个 target，**不要全 161 轴铺满**。源集硬编码成一个 `const`/小构造函数即可，禁动态全轴扫描。

**新增/修改签名草案：**
- `src/viewport.rs`：`push_prior` 内加 clamp。新增常量（建议放 `lib.rs` 或 viewport 顶部）`pub const MAX_PRIOR_MAGNITUDE: f64 = 100.0;`，改为：
  ```rust
  pub fn push_prior(&self, axis: usize, prior: f64) {
      let v = if prior.is_finite() {
          prior.clamp(-MAX_PRIOR_MAGNITUDE, MAX_PRIOR_MAGNITUDE)
      } else { 0.0 };
      self.prior_signal[axis].store(v.to_bits(), Ordering::Relaxed);
  }
  ```
- `src/vpc_cross_layer.rs`：（可选）加一个 body 侧构造函数 `pub fn axis_predictor_bank() -> Vec<CrossLayerPredictor>`，内部用上述稀疏先验表构建一组 `CrossLayerPredictor::new(target, sources)`，供 body 调用，把"选哪些轴"集中在一处、便于审阅其稀疏性。
- `src/bin/body.rs`：持有 `Arc<Mutex<Vec<CrossLayerPredictor>>>` + 在选定节律点 tick。顺手修正 :333 陈旧注释 "AXIS_COUNT (34)"。

**持久化（涉及）：** `CrossLayerPredictor` 持有学习到的权/bias/EMA，跨重启应保留，否则每次重启高层从零 warmup、prior 短暂归 0，破坏自洽连续性。已有 `SlotCrossLayerState` + `slot_xlayers` 快照块（`src/snapshot.rs:525,999–1139`）是读写对称 + 版本门控的范本。为 axis 版加一个对称的快照块：
- 给 `CrossLayerPredictor` 加一个可读快照结构（如 `AxisCrossLayerState { target_axis, source_axes, source_weights, bias, err_pred_mse, target_err_var, n_updates, ... }`）+ `fn to_state()/from_state()`，**纯数值，无字符串**。
- 在 `snapshot.rs` 写端（参照 :231）与读端（参照 :1025）成对加块，`SNAPSHOT_VERSION` 自增 **56→57**，读端用 `if version >= 57 { ... } else { Vec::new() }` 向后兼容旧档。**写多少字段，读端必须按同序读回多少**——务必读写对称。

### 实现计划（有序步骤）

1. **加 prior 幅度护栏（最小独立改动，先做）**：在 `src/lib.rs`（或 viewport 顶部）加 `pub const MAX_PRIOR_MAGNITUDE: f64 = 100.0;`；改 `src/viewport.rs:3339` `push_prior` 加 clamp。
2. **集中稀疏源先验**：在 `src/vpc_cross_layer.rs` 加 `axis_predictor_bank()`，硬编码一组 `(target, sources)` 稀疏对（先 ≤ 几十个），逐个 `CrossLayerPredictor::new`。审阅确认每个 target 的源是显式稀疏集、无全轴扇入。
3. **body 接线**：`src/bin/body.rs` main 里持有 `Arc<Mutex<Vec<CrossLayerPredictor>>> = Arc::new(Mutex::new(axis_predictor_bank()))`（若有快照则从快照恢复）；在 `run_global_refresh_cycle` slot/tower 之后插 tick 循环。修正 :333 陈旧注释。
4. **守卫纳管（必做，见下）**：把 `src/vpc_cross_layer.rs` 加进 `tests/body_no_human_output.rs` 的 `sources` 列表。
5. **持久化**：给 `CrossLayerPredictor` 加 `to_state()/from_state()` + 快照结构；`snapshot.rs` 写/读成对加块；`SNAPSHOT_VERSION` 56→57，读端版本门控；在 `lib.rs` 加一行 `/// v57:` 版本注释。
6. **编译 + 测试 + 守卫**（见验收）。
7. **服务器部署注意**：body 改动需服务器端 `cargo` 重新编译（先 `source ~/.cargo/env`）；`deploy_aborn.sh` 只同步不编译；网关重启用 `sudo -n`。本会话若只在 Mac 开发，至少本地 `cargo build` 通过。

### 验收标准（如实观察数值，不设功能靶子）

**原则护栏测试（必做）：**
- 把 `"src/vpc_cross_layer.rs"` 加入 `tests/body_no_human_output.rs:41` 的 `sources` 数组（当前**缺失**——它已是 body 传递依赖却未被扫描）。然后 `cargo test --test body_no_human_output` 必须通过（确认无 `println!/eprintln!/writeln!/write!/dbg!/format!/panic!/expect`）。

**已有 VPC 测试不得回归：**
- `cargo test --test vpc_cross_layer --test vpc_learning_progress`（确认学习律与噪声拒绝仍成立）。

**新增"如实观察闭环已接电"测试**（不写成"验证信用分配功能涌现"，而是如实断言数值动态）：
- 构造一个可学习 target 轴 + 其源轴有真实相关、再加一个纯噪声源；驱动若干拍后**如实观察**：(a) `vp.read_prior(target)` 由 0 变为非 0 有限值且 `|read_prior| <= MAX_PRIOR_MAGNITUDE`；(b) 噪声源的 `weight_for(noise_axis)` 维持在近 0（噪声拒绝）；(c) 接 prior 后 target 轴 err 的 EMA **低于**不接 prior 的对照轨。可参照 `tests/vpc_cross_layer.rs` / `tests/vpc_prior_injection.rs` 的搭法。
- clamp 测试：直接 `vp.push_prior(axis, 1e9)` 后 `read_prior(axis)` 等于 `MAX_PRIOR_MAGNITUDE`；`push_prior(axis, f64::NAN)` 后为 0。
- 快照往返：构造带非零权/bias 的 bank → 写快照 → 读回 → 字段逐一相等（读写对称）；旧版本档（version < 57）读回为空 bank、不 panic。

**命令：**
```
cd /Users/joe/unnamed/bionic_v0
cargo build
cargo test --test body_no_human_output --test vpc_cross_layer --test vpc_learning_progress
cargo test            # 全量回归，重点看 snapshot 往返
```

**对运行 body 的观察判据（外部 observer，禁让 body 自报）：** 接通前每轴 `read_prior` 恒 0；接通后，对覆盖的 target 轴，外部读二进制快照应能观察到 `prior_signal[target]` 出现非 0 有限值（warmup 后），且对应轴 err_stream 的 EMA 随时间下行。**如实描述观察到的数值曲线即可，不预设它"应该"达到某阈值。**

### 反模式与陷阱（逐条说改用什么）

1. **用 `println!/dbg!` 调试 prior 是否非 0** → 违推论1。改用：外部 observer 读快照，或在 **test**（允许字符串）里断言 `read_prior`。本体内一律不准。
2. **把 prior 折进 AR-2 梯度**（误改 `predictor.rs:80–81` 让 `used_x0/used_x1` 含 `external_prior`）→ 这就是跨层 BP，违 VPC 命门。保持加性、不折梯度。
3. **给 prior 通道挂 reward / 正负价**（例如"err 降就加正 prior 奖励高层"）→ 违推论5。prior 只是高层对低层 err 的预测值回写，无价标签；价值感应纯涌现自 err。
4. **高层直接读/写低层 predictor 的 a/b 或别的 predictor 的权** → 违"禁跨 predictor 直连"。只许经 `latest_err`/`latest_observation`/`push_prior`。
5. **全 161 轴铺满 + 全轴互为源（全 fan-in）** → 伪装的全局耦合/全局损失。改用：显式稀疏源集（`axis_predictor_bank` 里硬编码的 2–4 源/target），初版只接十几到几十轴。
6. **把"接通哪些轴"做成按能力/成熟度开门的资格门**（capability gate）→ 违推论4，是在为行为设靶子/造功能门。改用：固定稀疏先验表，无条件常驻 tick；该不该出 prior 由 predictor 自身的 `explained` 门（噪声→0）涌现决定，不由外部资格门控。
7. **让 axis 版去监听 slot 的 `object_err` 或复用 `gated_sources`** → err 域错配。axis 版只监听 `err_stream`（`latest_err`），源是显式轴集。
8. **忘记给 `push_prior` 加 clamp**，依赖 predictor 的 `clip=10.0` → 那只 clamp 自身权/bias，乘积 `prior_gain*explained*prior` 仍可超界。必须在 `push_prior` 收口到 ±100 量级。
9. **body 主循环里靠外部调用/请求驱动 tick** → 违推论3。必须挂在 body 自激主循环按固有节律自跑。
10. **快照只加写端不加读端（或字段顺序不对称）** → 读回错位/panic。写多少按同序读多少，并用 `if version >= 57` 门控旧档；`SNAPSHOT_VERSION` 必须自增。
11. **body.rs import 脚手架来选源轴**（curriculum/governor）→ 违铁边界。源轴表必须是 body 侧的纯数值常量，不经脚手架。
12. **高层 tick 频率 ≥ 低层 AR-2（100Hz）** → 高层比它要解释的 err 刷新还快，学不稳。挂在全局刷新串行节律（慢于 100Hz）。

---

## A3 — 粗→细精度退火接进预测回路并跨重启继承

> **速览** · 独立性：**与 A2 共享精度门限改面、与 B2 共享同一发育时钟 `developmental_maturity`**。建议在 A2 之后、与/在 B2 协调下进行。 · 前置依赖：无硬代码依赖，但强烈建议 A2 先落地（避免双方都改 predictor/vpc 门限造成合并冲突）。 · 预计触碰文件：`src/lib.rs`、`src/learned_bias.rs`、`src/viewport.rs`、`src/bin/body.rs`、`src/predictor.rs`、`src/vpc_cross_layer.rs`、`src/snapshot.rs`。 · 版本：`LEARNED_BIAS_VERSION` 4→5；`SNAPSHOT_VERSION` 51→52（**注意：起草时观察到的最高 snapshot 门是 51；A1/A2 已把 SNAPSHOT_VERSION 推到 56→57，接手时务必复核当前实际值再自增，不要盲信 51/52**）。

> 本提示词完全自包含。可据此冷启动直接干活。项目根：`/Users/joe/unnamed/bionic_v0`（务必用绝对路径操作文件）。（先粘贴上方【通用前言】。）

### 任务目标（补一条悬空的基质动态）

大脑里**已经存在一个发育时钟** `Viewport::developmental_maturity ∈ [0,1]`，它的设计意图（已写进记忆与源码注释）是统一调度全身的"精度战争"——**幼态宽容（粗，易学易发声，避免 0×x=0 自举死锁）→ 成熟收紧（细，保留精细辨别）**。但这条动态当前**三处断裂、悬空打转**：

1. **每次重启归零**：maturity 是 `RwLock<f64>`，runtime-only，进程一退就回到 0.0。死后不保留 → 每次启动都退回最幼态，发育从未真正累积（与"死后保留/factory_dna 可继承"的全局自洽矛盾）。
2. **涨了却没人用**：它在运行时**确实被推进/累积**（活路径见下），但运行时**没有任何活消费者**读它——唯一读它的函数是死代码（仅测试触达）。即"在跑时会涨、但没人用、且重启即归零"。
3. **退火机制悬空**：记忆明确"学习曲线=粗→细精度退火"，但 AR-2 预测器与 cross-layer 拱心石的精度/容差门限**全是写死常数**，没有一处随这个发育时钟调度。退火作为全局自洽机制处于悬空。

本任务 = **把这条已存在但断裂的发育动态闭合成一个完整回路**：让它（a）由真实 closure 推进（已成立，确认即可），（b）跨重启继承，（c）真正回流到 AR-2 / cross-layer 的本地精度门限上，让"何时该粗、何时该细"由同一个发育时钟统一裁决。

**这不是给某个行为设靶子**。我们不验证"发声涌现"或"语言涌现"，只补全这条动态的自洽，行为自会涌现（或不涌现）。

**核心定性铁律（违反即推翻整个任务）**：maturity 是**全局发育标量**，是一条平铺在所有轴上的精度退火曲线。**绝不能**把它做成"按能力名逐层开门的资格门"（如"成熟到 X 才解锁发声/绑定/某轴"）——那违反并列原则与推论4。它只调**连续的本地标量门限**（lr/clip/容差/explained 门），从不做布尔开关、从不按能力命名。

### 背景与现状（带 file:line 实据）

所有事实已逐行核实。绝对路径前缀 = `/Users/joe/unnamed/bionic_v0/`。

**状态字段本体（BODY 层，`src/viewport.rs`）**
- **字段** `src/viewport.rs:1334`：`pub developmental_maturity: RwLock<f64>`，注释 `:1329-1333` 含 `TODO: persist into learned_bias.bin so maturity is inheritable via checkpoint (factory_dna)`。
- **初始化** `src/viewport.rs:1442` 附近，在 `Viewport::new()`（`:1347` 起）内 `developmental_maturity: RwLock::new(0.0)`。`with_birth_sensitivity` 复用 `new()`，不另设。
- **读取器** `src/viewport.rs:2179-2181`：`pub fn developmental_maturity(&self) -> f64`（裸读锁）。
- **唯一推进器** `src/viewport.rs:2187-2194`：`pub fn advance_maturity(&self, closure_support: f64)`。语义：`c = clamp(0,1)`；`c<=0` no-op；`*m = (*m + c·MATURITY_GAIN·(1−*m)).clamp(0,1)`（饱和、单调、≤1）。**没有任何 setter**——这是唯一写入口（持久化回填时需新增一个受控 setter，见下）。

**maturity 当前推进（producer）—— 活路径已确认**
- **活的**：`src/viewport.rs:4090`，在 `observe_vocal_reflex_closure(now_us, closure_support, vocalized)`（`:4069-4091`）末尾无条件 `self.advance_maturity(closure_support)`。它被两条活路径触达：
  - `tick_vocal_reflex_from_pairs`（`:3963` 附近）→ `src/bin/body.rs:164` 附近 `vp.tick_vocal_reflex_from_pairs(t)`。**这是确认接进运行时的链。**
  - VIB 自玩路径（`self_match > 0.85` 且过节流窗）`observe_vocal_reflex_closure(now_us, support, true)`。
  → **结论**：maturity 已由真实 closure（自玩 + 看护者应答）推进，**不是 wall-clock**。本任务（3）只需确认这点、不要改成按时间走。
- **死的 producer**：`src/language_binding.rs:229`（`try_form_binding_event_from_pair` 内）也调 `advance_maturity`，但该函数全仓唯一调用方是 `tests/language_binding_runtime.rs`，`src/bin/body.rs` 不调用 → 运行时死代码。

**maturity 当前消费（consumer）—— 运行时无人消费**
- **唯一读取点**：`src/language_binding.rs:197-199`，用 `binding_align_min(m)`/`binding_deviate_max(m)` 做 Align/Deviate 阈值。但它在 `try_form_binding_event_from_pair` 内 → **运行时死代码**（同上）。
- 全仓引用 maturity 的文件仅 5 个：`src/lib.rs`、`src/language_binding.rs`、`src/viewport.rs`、`tests/language_binding_runtime.rs`、`tests/developmental_bootstrap.rs`。

**关键常数（`src/lib.rs`）**
- `:569` `pub const MATURITY_GAIN: f64 = 0.01`。
- `:550` `BINDING_PAIR_ALIGN_MIN = 0.70`；`:555` `BINDING_PAIR_DEVIATE_MAX = 0.30`；`:579` `BINDING_PAIR_NEUTRAL = 0.50`。
- `:583-586` `binding_align_min(m) = 0.50 + (0.70−0.50)·m`；`:590-593` `binding_deviate_max(m) = 0.50 − (0.50−0.30)·m`。**这是「已随 maturity 由宽→紧线性插值」的唯一现成先例，新增门限调度仿此形式。**
- `:573` `VOCAL_SELF_PLAY_MIN_INTERVAL_US = 250_000`。

**可挂靠的持久化载体（`src/learned_bias.rs`，已读全文）**
现成二进制死后保留载体，模式：
- `:7` `LEARNED_BIAS_MAGIC = 0xB10B10_0000_0042`；`:8` `pub const LEARNED_BIAS_VERSION: u64 = 4`。
- `LearnedBiasState`（`:57-72`）当前尾部三个 f64 标量：`prediction_familiarity`/`action_suppression`/`vocal_efficacy`（`:65/:66/:71`）。**`LearnedBiasState` 不含 maturity 字段。**
- 写 `write_learned_bias_checkpoint`（`:239-269`）：header → curves → events → `total_events`/`last_update_t_us` → 三个尾部 f64（`:258-260`）。helper `write_f64`（`:445` 附近，LE 8 字节）。
- 读 `read_learned_bias_checkpoint`（`:271-319`）：version 严格 `== LEARNED_BIAS_VERSION`（`:279`，否则 `InvalidData`）→ 顺序读回三个尾部 f64（`:304-306`）。
- v4 注释（`:256-257`/`:303`）即"把一个新 f64 标量挂上去并死后保留"的现成先例。

**body.rs 的 load/save 生命周期（接 maturity 须触达这三处）**
- **boot load** `src/bin/body.rs:200-202`：`if let Ok(state) = read_learned_bias_checkpoint(&learned_bias_paths.final_path) { *vp.learned_bias.write().unwrap() = state; }`。注意只回填 `vp.learned_bias`；**maturity 是 Viewport 上的独立字段**，即便序列化进同一 .bin 也需在此额外把值写回 `vp.developmental_maturity`。
- **周期 save** `src/bin/body.rs:462-494`（checkpoint ticker，`profile.checkpoint_hz`）：`:473` `let state = vp_checkpoint.read_learned_bias();` → `:474` `write_learned_bias_checkpoint`。`read_learned_bias()` 签名见 `src/viewport.rs:2162`，返回 `LearnedBiasState` 的 clone。
- **exit save** `src/bin/body.rs:538-539`：同样 `read_learned_bias()` → `write_learned_bias_checkpoint`。
- 关键障碍：写盘只取 `read_learned_bias()` 返回的 `LearnedBiasState`，**看不到 `Viewport.developmental_maturity`**。要走这个载体，必须让 maturity 流进 `LearnedBiasState`（推荐做法，见架构设计）。

**可随 maturity 由宽→紧调度的本地门限（已读使用点）**
- **`src/predictor.rs`（AR-2 轴级，每轴本地 Widrow-Hoff）**：
  - `Ar2Predictor::new`（`:27-38`）实例字段 `base_lr: 2.0`（`:33`）、`clip: 10.0`（`:34`）、`horizon_us: 10_000`（`:35`）。
  - `tick(&mut self, vp: &Arc<Viewport>)`（`:40`）：`:79` `eff_lr = self.base_lr * surprise`；`:80-83` `da/db = eff_lr·err·used_x`，`a/b` clamp 到 `±clip`。`tick` **已持有 `vp`**，可直接 `vp.developmental_maturity()`。当前无任何门限随 maturity 变。
  - `:102-103` 外部 prior 仅作 input 相加（`pred = a·x_now + b·x_prev + external_prior`），不入 `used_x`（保 VPC 不跨层回传）。
- **`src/vpc_cross_layer.rs`（拱心石机制5：高层预测低层 err）**：
  - `CrossLayerPredictor::new`（`:107-126`）实例字段 `base_lr: 0.5`（`:117`）、`clip: 10.0`（`:118`）、`prior_gain: 1.0`（`:123`）；`const WARMUP_MIN: u32 = 8`（`:105`）。
  - `tick(&mut self, vp: &Arc<Viewport>)`（`:152`）：`:175-178` 本地 LMS（`bias`/`w` 用 `base_lr·meta_err`，clamp ±clip）；`:198-202` 噪声门 `explained = (1 − err_pred_mse/target_err_var).clamp(0,1)`；`:206-212` warmup 后 `vp.push_prior(target_axis, prior_gain·explained·prior)`。`tick` **已持有 `vp`**。
  - slot 级 `SlotCrossLayerPredictor` 另有 `base_lr: 0.3` 等（同模式）。
- **现成先例**：`binding_align_min`/`binding_deviate_max`（`lib.rs:583/590`）—— m=0 塌到中点（粗判也给信用），m=1 张到严格。新增 predictor/VPC 门限调度仿此线性插值。

**观察快照（`src/snapshot.rs`）**
- `grep maturity/developmental` 在 `src/snapshot.rs` **零命中** → observer 当前看不到 maturity。
- 版本机制：`:38` 引入 `SNAPSHOT_MAGIC, SNAPSHOT_VERSION`；写头 `:60-61`；读 version 严格 `==`（`:728-729`）。起草时最高版本门为 **v51**（`:1041` `if version >= 51`）。**接手时复核 `SNAPSHOT_VERSION` 当前实际值（A1/A2 可能已推进），在其上自增**，写/读对称，老版本读路径用 `version >= 新版本` 取默认。

### 铁律与边界（本任务真正会碰到的红线）

- **守卫测试 `tests/body_no_human_output.rs`**（扫描表 `:41-90`）**已含** `src/bin/body.rs`(:43)、`src/predictor.rs`(:45)、`src/viewport.rs`(:47)、`src/snapshot.rs`(:49)、`src/learned_bias.rs`(:82)。这些文件里**禁** `println!/eprintln!/writeln!/write!/dbg!/expect/format!/panic!`。错误路径沿用 learned_bias.rs 现风格：`?` + `std::io::Result`；body.rs 的 load 用 `if let Ok(..)` 静默忽略失败（推论3：任一线程不得 panic 让本体半死）。
- **不进观察通道做人类可读**：maturity 是 tier-0 数值。可写进二进制观察快照（observer 读二进制，推论2），但**不得转人类可读**、不得 log。
- **VPC 硬约束**：接进 predictor.rs / vpc_cross_layer.rs 时**只能调本地标量**（`base_lr`/`clip`/`explained` 门/`prior_gain`/容差），**禁**借此引入跨层链式/全局损失；prior 写回须继续走既有 `push_prior`/`read_prior`（predictor.rs:102、vpc_cross_layer.rs:211），**绝不动**别层的 weights/bias。
- **VERSION bump（learned_bias.bin）**：复用该载体须 `LEARNED_BIAS_VERSION` 从 4→5 并保持 read/write 对称。现 read 对 version 不等即 `InvalidData` → boot 落回 `Viewport::new()` 默认 0.0，**等于一次"新生"**（可接受的退化，但要知晓——旧 .bin 会被拒，body 从最幼态重新发育）。
- **新生默认不可破**：任何回填/setter 须保持 `Viewport::new()` 后、且无 .bin 时 maturity==0.0（测试 `developmental_bootstrap.rs:23/:57` 断言新生为 0）。
- **行为契约不可破**（改实现须不破）：
  - `tests/developmental_bootstrap.rs:21-40`：maturity 单调、饱和、≤1；1000 次 `advance_maturity(1.0)` 后 `>0.99`；`advance_maturity(0.0/-1.0)` no-op。`:42-51`：`binding_align_min(0)==binding_deviate_max(0)`（无死区）、`binding_align_min(1) > binding_deviate_max(1)+0.3`、随 m 单调收紧。`:53-86`：纯自玩抬升 efficacy/familiarity/maturity。
  - `tests/language_binding_runtime.rs:207/225/230`：新生 vs `advance_maturity(1.0)` 后成熟体的 binding 行为对照。
- **body.rs 不得 import 脚手架**：禁 import/调用 `curriculum.rs`/`curriculum_governor.rs`/`capability_curriculum_governor.rs`/`five_phase_life.rs`。（注：body.rs 已 import `evolution_priors` 的 `BiasPriors`——那是 DNA 偏置载体，**允许**，不是 curriculum scaffolding。）

**绝对不要改动**
- `advance_maturity` 的累积语义（`viewport.rs:2187-2194`）与 `MATURITY_GAIN`（除非有强理由，本任务不动）。
- maturity 的推进来源——**保持由 closure 推进，绝不改成 wall-clock/tick 计数**。
- `binding_align_min`/`binding_deviate_max` 的现有形状（它们是先例，照抄形状给新门限用，但不动它们本身的常数）。
- 任何把 maturity 变成布尔/能力开关的改写。

### 架构设计（让已有结果回到循环，不新建模块）

三段闭合，全部复用现有载体与 tick，不新增模块、不新增线程、不新增驱动入口（推论3：本体自激，禁外部调用驱动）。

**A. 持久化（让 maturity 死后保留、可继承）—— 复用 learned_bias.bin**
推荐方案：把 maturity 作为第 4 个尾部 f64 挂进 `LearnedBiasState`，随既有三标量一同读写。这样周期/exit save 自动带上它，无需在 body.rs 三处各写一遍盘。
- 在 `src/learned_bias.rs`：
  - `LearnedBiasState`（`:57-72`）新增字段 `pub developmental_maturity: f64`（默认 0.0；`#[derive(Default)]` 已在，新字段自动 0.0 → 守住新生默认）。
  - `LEARNED_BIAS_VERSION` 4→5（`:8`）。
  - `write_learned_bias_checkpoint`（`:260` 之后）追加 `write_f64(&mut buf, state.developmental_maturity);`。
  - `read_learned_bias_checkpoint`（`:306` 之后）追加 `let developmental_maturity = read_f64(&bytes, &mut cur)?;`，并放进返回的 struct（`:308-318`）。
- 在 `src/viewport.rs`：
  - `read_learned_bias()`（`:2162`）：让它把当前 `developmental_maturity()` 装进返回 `LearnedBiasState.developmental_maturity`，这样周期/exit save 自动带上（save 端 body.rs 无需改）。
  - 新增**受控 setter**用于 boot 回填，例如：
    ```rust
    /// Restore the developmental clock from a checkpoint. Boot-only; clamps to
    /// [0,1]. Keeps newborn default 0.0 when no checkpoint exists (advance_maturity
    /// remains the only forward chokepoint during life).
    pub fn restore_maturity(&self, m: f64) {
        *self.developmental_maturity.write().unwrap() = m.clamp(0.0, 1.0);
    }
    ```
- 在 `src/bin/body.rs` boot load（`:200-202`）：在回填 `vp.learned_bias` 之后，额外 `vp.restore_maturity(state.developmental_maturity);`（仅当 `read_..._checkpoint` 成功的 `if let Ok` 分支内）。save 两处（`:462-494`、`:538-539`）**无需改**，因为 `read_learned_bias()` 已带上 maturity。

> 备选方案（独立 `developmental.bin`）：仿 phoneme_memory.bin 模式（body.rs:470/487-488）。可行但要在 boot/周期/exit 三处各加读写，且新增 .bin 文件须进守卫扫描表——比挂 learned_bias.bin 多三处接线。**优先用 learned_bias.bin 方案。**

**B. 消费：把 maturity 接进 AR-2 精度退火（`src/predictor.rs`）**
在 `Ar2Predictor::tick`（已持有 `vp`）内读 `let m = vp.developmental_maturity();`，让**有效学习率/容差由 m 由宽→紧线性插值**：
- 幼态（m→0）：**更大 eff_lr / 更宽 clip**（粗、快、易学，给 0×x≈0 的初值一个非零起步），可仿 `binding_*` 形状，例如 `lr_scale(m) = LR_SCALE_NAIVE + (1.0 − LR_SCALE_NAIVE)·m`（m=0→放大，m=1→1.0），`eff_lr = self.base_lr * surprise * lr_scale(m)`。
- 成熟（m→1）：回到现有写死值（精细辨别）。
- 新增的插值常数放 `src/lib.rs` 的 "Developmental precision schedule" 段（`:557` 起，紧挨 `binding_*`），写成纯函数 `pub fn ...(maturity: f64) -> f64`，便于测试与复用。**不动 `predictor.rs` 里的 `base_lr` 初值本身**，只乘一个 m 调度系数。
- **红线**：`external_prior` 仍只作 input、不入 `used_x`（保持 `:99-103` 的 VPC 局部性）。

**C. 消费：把 maturity 接进 cross-layer 精度退火（`src/vpc_cross_layer.rs`）**
在 `CrossLayerPredictor::tick`（已持有 `vp`）内读 `m`，调度本地标量：
- 让 `explained` 噪声门或 `prior_gain` 随 m 由宽→紧：幼态时 prior 写回更宽容（让高层敢于早早提供粗 prior 帮低层降 err，缓解自举死锁），成熟时收紧到现有严格门（避免过度自信）。例如对 `:211` 的写回乘一个 `gain_schedule(m)`，或对 `explained` 的下限随 m 抬升。
- 同样把插值常数放 `lib.rs`，纯函数形式。
- **红线**：仍只走 `vp.push_prior`（`:211`）；本地 LMS（`:175-178`）只调 `base_lr` 的 m 系数，**禁**引入跨层梯度。

**节律与耦合**
- 不新增 tick：A 走既有 boot/checkpoint/exit；B/C 在既有 `Ar2Predictor::tick`、`CrossLayerPredictor::tick` 内顺带读 m（每 tick 一次 `RwLock` 读，开销可忽略）。
- 统一时钟：B、C、以及既有 `binding_*` 全部读**同一个** `developmental_maturity()` → "何时该粗/何时该细"由单一发育标量统一裁决（精度战争"周期收紧"落地点）。
- 闭环：closure → `advance_maturity`（已有）→ 收紧 lr/容差/prior 门 → 预测更精细 → 后续 err 结构变化 → 再影响 closure。让已有结果回到循环，而非新建通路。

### 实现计划（有序，逐步指明目标文件与改动点）

1. **`src/lib.rs`**：在 "Developmental precision schedule" 段（`:557` 起）新增 B、C 的插值纯函数与常数（仿 `binding_align_min` 形状）。例如 AR-2 的 `pub fn ar2_lr_scale(maturity: f64) -> f64`、cross-layer 的 `pub fn vpc_prior_gain_scale(maturity: f64) -> f64`（命名按你定，但**禁**用能力名）。保证 m=0 给宽容值、m=1 回到 1.0（即"不改写死值"）。
2. **`src/learned_bias.rs`**：`LEARNED_BIAS_VERSION` 4→5（`:8`）；`LearnedBiasState` 新增 `pub developmental_maturity: f64`（`:57-72`）；`write_learned_bias_checkpoint` 在 `:260` 后追加一个 `write_f64`；`read_learned_bias_checkpoint` 在 `:306` 后追加一个 `read_f64` 并放进返回 struct（`:308-318`）。（可选）`LearnedBiasSummary`（`:74-97`）若 observer 经它读，也加对应字段——视快照是否走 summary 而定。
3. **`src/viewport.rs`**：新增 `restore_maturity(&self, m: f64)`（boot-only，clamp 到 [0,1]）；`read_learned_bias()`（`:2162`）把 `developmental_maturity()` 装进返回的 `LearnedBiasState.developmental_maturity`。
4. **`src/bin/body.rs`**：boot load（`:200-202`）的 `if let Ok(state)` 分支内，回填 `vp.learned_bias` 后追加 `vp.restore_maturity(state.developmental_maturity);`。save 两处不动。
5. **`src/predictor.rs`**：`Ar2Predictor::tick` 内 `let m = vp.developmental_maturity();`，用步骤1的 `ar2_lr_scale(m)` 调 `eff_lr`（`:79` 附近）/（可选）clip。
6. **`src/vpc_cross_layer.rs`**：`CrossLayerPredictor::tick` 内读 m，用步骤1的调度函数调 prior 写回门（`:211`）或 `explained` 下限 / `base_lr`。
7. **`src/snapshot.rs`（观察通道，二进制）**：`SNAPSHOT_VERSION` 自增（复核当前值再 +1）；写端把 `vp.developmental_maturity()` 作 1 个 LE f64 写入；读端 `if version >= 新版本 { rd_f64 } else { 默认 0.0 }`，写/读严格对称。**不转人类可读**。

### 验收标准（如实观察数值，不设功能靶子）

只跑测试 + 观察数值动态，不"验证 X 功能涌现"。

1. **持久化往返（新增测试，建议 `tests/developmental_bootstrap.rs` 内扩展或新建）**：构造一个 `LearnedBiasState`，置 `developmental_maturity = 0.37`，`write_learned_bias_checkpoint` 到 temp 目录再 `read_..._checkpoint` 回来，**如实断言读回值 == 0.37**（往返无损、字段对齐）。再断言：读一个 v4 旧字节（version 不等）返回 `InvalidData`（即触发新生退化，符合预期）。
2. **boot 回填 + 新生默认**：断言无 .bin 时 `Viewport::new().developmental_maturity() == 0.0`（守住 `:23/:57` 契约）；`restore_maturity(0.5)` 后读回 0.5；`restore_maturity(-1.0)`→0.0、`restore_maturity(2.0)`→1.0（clamp）。
3. **既有契约回归**：`developmental_bootstrap.rs`（单调/饱和/无死区/自玩抬升）与 `language_binding_runtime.rs`（新生 vs 成熟 binding 对照）**全绿不改语义**。
4. **退火调度纯函数**：对步骤1新增的 `ar2_lr_scale`/`vpc_prior_gain_scale` 断言：m=0 给宽容值、m=1 ≈1.0、随 m 单调（仿 `binding_*` 的 `:42-51` 测试形态）。
5. **观察某轴的退火动态（如实观察，非靶子）**：可在测试里建 `Viewport`，对某轴喂同一序列样本，分别在 `restore_maturity(0.0)` 与 `restore_maturity(1.0)` 下跑若干 `Ar2Predictor::tick`，**如实记录两种成熟度下 `eff_lr` / 系数 a,b 的收敛轨迹差异**（幼态应更快/更糙地动，成熟应更稳）。断言"幼态的有效学习率 > 成熟态"这一**方向性数值关系**，不断言任何行为达成。
6. **原则护栏**：确认本任务**未新增 body 模块**（复用现有文件）；若你新增了任何 BODY 层 `.rs`，必须把它加进 `tests/body_no_human_output.rs` 的扫描表（`:41-90`）。跑守卫测试，确认 learned_bias.rs / viewport.rs / predictor.rs / vpc_cross_layer.rs / snapshot.rs / body.rs 无 `println!/eprintln!/writeln!/write!/dbg!/expect/format!/panic!`。
7. **编译 + 全测**：
   ```bash
   cd /Users/joe/unnamed/bionic_v0
   cargo build
   cargo test developmental_bootstrap
   cargo test language_binding_runtime
   cargo test body_no_human_output
   cargo test            # 全量回归，确认无破坏
   ```

### 反模式与陷阱（逐条说改用什么）

- **把 maturity 做成"按能力开门的资格门"**（如"m>0.6 才允许 binding/发声/解锁某轴"）：违反并列原则 + 推论4。→ 改：只用它对**连续本地标量**（lr/clip/容差/explained/prior_gain）做线性插值，永不做布尔/能力开关、永不按能力命名。
- **用 `println!/dbg!` 在 body 模块里调试**：守卫测试会红。→ 改：写进二进制观察快照，让 observer 读；调试期只在 `tests/*.rs`（允许字符串）里断言。
- **boot 回填或持久化路径用 `unwrap/expect/panic!`**：违反守卫 + 推论3。→ 改：`?` 传播 `std::io::Result`，body.rs 用 `if let Ok(..)` 静默忽略失败。
- **改 schema 不 bump `LEARNED_BIAS_VERSION`**：旧/新字节错位读，静默数据腐坏。→ 改：4→5，read/write 严格对称。
- **save 端在 body.rs 三处各手写 maturity 落盘**：易漏一处导致不对称。→ 改：让 `read_learned_bias()` 带上 maturity，save 自动覆盖。
- **把 maturity 改成 wall-clock / tick 计数推进**：违反"由真实 learning_progress/closure 推进"。→ 改：保持 `advance_maturity` 由 closure 调用（已成立），本任务不碰推进源。
- **在 predictor/vpc 里借 maturity 引入跨 predictor 直读/写 θ 或全局损失**：违反 VPC 硬约束。→ 改：只调本层本地标量；跨层只经 `push_prior/read_prior/latest_err`；绝不动别层 weights/bias。
- **maturity 转 JSON/文本/人类语义标签进数据通路**：违反推论1。→ 改：始终是裸 f64，二进制读写。
- **新增线程/外部调用来"驱动 maturity 调度"**：违反推论3（本体自激）。→ 改：在既有 `tick` 内顺带读 m，不新建驱动入口。
- **m=1 时把 lr/门限调得比现有写死值更松/更紧而改变现有成熟态行为**：会破 `language_binding_runtime` 等对照。→ 改：调度函数在 m=1 处必须回到现有写死值（系数 ≈1.0），只在 m<1 时放宽。
- **破坏新生默认（`Viewport::new()` 后 maturity≠0）**：破 `:23/:57` 契约。→ 改：新字段靠 `Default` 取 0.0；`restore_maturity` 仅在 `if let Ok` 成功分支调用；无 .bin 时绝不调。

---

# 组 B · 永跑地基（守住"永远在跑"）

> 两个提示词都属"消除会让本体静默半死/锁死的结构缺陷"。它们是地基：在灵魂接通（组 A）之前先把"永远在跑"的工程保证打牢，能让组 A 的观察更可靠。

---

## B1 — 加固"永远在跑"的鲁棒性（推论3 工程保证）

> **速览** · 独立性：**完全独立**，无前置依赖，是最适合最先做的低风险地基。 · 前置依赖：无。 · 预计触碰文件：`src/clock.rs`、`src/bin/body.rs`、`src/adapter.rs`、`src/net_adapter.rs`、`src/viewport.rs`、`src/snapshot.rs`、`tests/body_no_human_output.rs`、（评估后或）`Cargo.toml`。 · 快照版本：`SnapshotData` 版本 +1（复核当前值再自增）。

> 你将在 **零上下文** 的会话里独立完成这项任务。项目根：`/Users/joe/unnamed/bionic_v0`（源码扁平在 `src/` 下，没有 `bionic_v0/bionic_v0/...` 嵌套）。下文给出的 file:line 来自一次代码核实，但你接手后必须自己再读一遍确认，不要盲信行号。（先粘贴上方【通用前言】。）

### 任务目标（这条补全闭合大脑的哪条自洽缺环）

本体是一台共享同一个 `Arc<Viewport>` 的多节律自激机器：十余条 ticker 线程（预测、高层、健康、摄入、绑定、内生、发声、编排、master clock 等）各自按节律读写同一份 viewport 状态。当前这台机器有一条**悬空的自洽缺环**：任何一条 ticker 线程内的一次未预期 panic 会沿 `thread::spawn` 直接 unwind 杀死该线程，而 `stop` 标志仍为 `false`，主线程**从不检查线程存活**——于是该节律永久停摆，但进程不退、其余线程继续跑。更糟的级联路径：viewport 里大量状态用 `std::sync::RwLock`/`Mutex` 守护，body 层靠 `.unwrap()` 解锁；一条持锁线程 panic 会**毒化（poison）该锁**，其它线程下次对同锁 `.unwrap()` 时连锁 panic，多节律线程在共享 viewport 上**雪崩式僵死**。结果就是推论3 明令禁止的"静默半死"。

这不是"加看门狗功能"，而是**补全"自激机器对自身节律存活的自洽闭环"**：机器目前能感知外部感官故障（已有 `hearing_fault` 轴），却对"自己的某条心跳停了"完全无感——这是一条和推论3 矛盾的空白动态。补全后，崩溃要么被就地吸收、节律继续自激，要么如实落到一条中立的 tier-0 故障计数轴上由 observer 读到。**不为此设行为靶子**：我们不验证"看门狗能修复 X"，只如实观察"线程崩溃这件事不再无声无息、本体不再能半死"。

### 背景与现状（带 file:line 实据，接手后自行复核）

**关键文件（绝对路径）**
- `/Users/joe/unnamed/bionic_v0/src/clock.rs` — 线程生成原语
- `/Users/joe/unnamed/bionic_v0/src/bin/body.rs` — 本体主进程，spawn 所有 ticker、主循环、join
- `/Users/joe/unnamed/bionic_v0/src/adapter.rs` — 传感器驱动线程
- `/Users/joe/unnamed/bionic_v0/src/net_adapter.rs` — headless 网关感官入口（含 4 个运行期 `expect`）
- `/Users/joe/unnamed/bionic_v0/src/viewport.rs` — 共享状态 + 已有 tier-0 故障轴范式
- `/Users/joe/unnamed/bionic_v0/src/snapshot.rs` — 二进制快照序列化（observer 读这个）
- `/Users/joe/unnamed/bionic_v0/tests/body_no_human_output.rs` — 推论1 守卫测试
- `/Users/joe/unnamed/bionic_v0/Cargo.toml` — 依赖

**clock.rs — 线程生成原语（无 catch_unwind）**
- `pub fn spawn_ticker<F>(rate_hz: u32, stop: Arc<AtomicBool>, mut f: F) -> JoinHandle<()> where F: FnMut(u64) + Send + 'static` — **clock.rs:54-79**。循环体 67-77：`while !stop.load(Relaxed) { f(local_tick); ... thread::sleep(...) }`。`f` 内 panic 直接 unwind 杀线程，`stop` 不变。**这是核心漏洞落点。**
- 注意 **clock.rs:62** period 计算 `1_000_000u64 / rate_hz as u64`——`rate_hz=0` 会除零 panic（master clock 用 `.max(1)` 防了，spawn_ticker 没防；当前调用方都传非 0 常量）。
- `pub fn spawn_master_clock_hz(vp: Arc<Viewport>, stop, rate_hz: u32) -> JoinHandle<()>` — **clock.rs:26-52**，同样裸 spawn，直接捕获 `Arc<Viewport>`。
- `pub fn now_us() -> u64` — clock.rs:12-17，已用 `.unwrap_or(0)` 防护。
- `pub struct ClockHandles { pub threads: Vec<JoinHandle<()>> }` — **clock.rs:8-10，死代码**（body.rs 不用，各 handle 单独持有）。

**body.rs — 主线程无运行期存活监控**
- 主循环 **body.rs:507-519**：有限时长则 `while elapsed < dur { sleep(100ms) }` 然后 `stop.store(true)`；否则 `while !stop.load(Relaxed) { sleep(100ms) }`。**只轮询 stop，从不检查任何 handle 存活。** 这是 watchdog 的天然落点（已经每 100ms 醒一次）。
- 退出后 **body.rs:521-536** 串行 `let _ = h_*.join();`（16 个 handle：h_surface, h_adapters, h_pred, h_hl, h_health, h_ingress, h_binding, h_endogenous, h_vocal, h_vocal_output, h_audio, h_learned_bias_checkpoint, h_orchestrator, h_live_byte, h_live_byte_closure, h_master）。全程**无 `is_finished()`**（全 src `is_finished` = 0 处）。
- ticker 闭包各捕获一个 `Arc<Viewport>` clone（vp_pred、vp_hl、vp_health、vp_ingress、vp_binding、vp_endo、vp_vocal、vp_orch、vp_checkpoint、vp_surf）。
- health ticker **body.rs:361-363**：`spawn_ticker(profile.health_hz, stop, move |_| vp_health.update_health_state(now_us()))`。
- 已有"失败→递增 tier-0 轴而非 print"先例：`run_global_refresh_cycle` 末尾 **body.rs:170-172** `if write_snapshot(vp, paths).is_err() { vp.mark_snapshot_fail(); }`。**照这个范式做**。
- 已有"致命错误用进程级动作而非文本"先例：`register_or_abort`（body.rs:175-179，`std::process::abort()`）、SIGTERM 注册失败（body.rs:257-259，`std::process::exit(1)`）。
- body.rs 15 处 `.unwrap()` 全是锁守卫；其中 ticker 闭包内的（约 body.rs:343/355/410）在锁中毒时会再次 panic → **级联杀更多 ticker**，是本任务要切断的链路。

**adapter.rs / 其它 IO 线程 — 同样裸 spawn**
- `spawn_registry_driver` — **adapter.rs:123-141**：`thread::spawn(move || { let mut reg = build(); while !stop.load(Relaxed) { reg.tick_due(&vp, t); sleep(2ms) } })`。`reg.tick_due` panic 静默杀整个传感器驱动（VIB+PHOT 全停）。
- `spawn_vocal_output`（vocal_adapter.rs:36）、`spawn_live_byte_ingress`（live_byte_carrier.rs:70）、h_audio（body.rs:435-445 裸 spawn loop）同类。

**net_adapter.rs — 4 个运行期 `expect`（推论1 残留红线，本任务顺手清除）**
在 `NetSensesAdapter::new` spawn 的 `"net-senses-reader"` 线程内：
- **net_adapter.rs:185** `UnixListener::bind(&socket_path).expect("net_adapter: bind Unix socket")`
- **net_adapter.rs:188** `listener.accept().expect("net_adapter: accept gateway connection")`
- **net_adapter.rs:191** `stream.try_clone().expect("net_adapter: clone socket for vocal sink")`
- **net_adapter.rs:209** `.expect("net_adapter: spawn reader thread")`

这是本体层**仅有的生产路径 `.expect(`**（4 个），带人类语言字符串 + panic 会杀 headless 唯一感官入口。它们正是 `net_adapter.rs` 至今未纳入守卫测试的原因。

**viewport.rs — 已有可复用的 tier-0 故障轴范式（照抄这个，别发明新形态）**
- `pub snapshot_fail_count: AtomicU64`（viewport.rs:1108，init `AtomicU64::new(0)`）；`pub fn mark_snapshot_fail(&self)` fetch_add（**viewport.rs:3377-3379**）；`pub fn snap_fail_count(&self) -> u64`（**3381-3383**）；唯一消费者 body.rs:171，写入快照 snapshot.rs:69。
- **"故障布尔 + 计数"双字段范式**（最贴近"线程崩溃"语义，照此新增）：
  - `pub hearing_fault: AtomicBool`（viewport.rs:1110）、`pub hearing_fault_count: AtomicU64`（viewport.rs:1111）
  - `pub fn set_hearing_fault(&self, fault: bool)`（**viewport.rs:3385-3387**）
  - `pub fn report_audio_error(&self)`：`hearing_fault.store(true)` + `hearing_fault_count.fetch_add(1)`（**viewport.rs:3389-3392**）
  - `pub fn hearing_status(&self) -> (bool, u64)`（**viewport.rs:3394-3399**）
- 健康枚举 `pub enum HealthState { Fault=0, Quiet=1, Ok=2, Stale=3 }`（viewport.rs:57-78）；`pub health_state: AtomicU8`（1116）；`update_health_state`/`compute_health_state`（3462-3492）目前是**纯感官健康，无"线程死亡"维度**。

**snapshot.rs — observer 读这里（写出新轴的落点）**
- `SnapshotData` 字段含 `snapshot_fail_count`(456)、`hearing_fault`(459)、`hearing_fault_count`(460)、`health_state: u8`(467)、`phot_health: u8`(468)（snapshot.rs:449-468）。序列化点 snapshot.rs:69-95，反序列化 snapshot.rs:743-758。**写/读必须对称**，且有版本号需自增（接手后在 snapshot.rs 顶部找 `VERSION`/`MAGIC` 常量，复核当前值再 +1）。

**Cargo.toml — 依赖现状**
- **未依赖 `parking_lot`**（全 src grep = 0）。现锁全是 std `RwLock`/`Mutex`，`.read()/.write()/.lock()` 返回 `Result`，body 靠 `.unwrap()` 解 → 持锁线程 panic 即毒化、传染。
- 现 deps：cpal 0.15、nokhwa 0.10、pixels 0.13、winit 0.28、rayon 1.10、signal-hook 0.3。
- 全 src `catch_unwind` = 0 处，`is_finished` = 0 处。

### 铁律与边界（本任务真正会碰到的红线）

1. **崩溃的健康表达只能是 tier-0 数值轴**（`AtomicU64`/`AtomicBool` 计数），由 snapshot.rs 写出、observer 读快照呈现。**绝对禁** `println!/eprintln!/writeln!/write!/dbg!`、禁 `panic!("...")`、禁任何文本日志、禁把 panic 的 payload 字符串（人类语言）写进任何 body 状态或文件。
2. **catch_unwind 捕获后禁恢复成"请求-响应/等指令重启"语义**。本体继续自激（推论3）：要么就地吞掉这一拍让该节律继续转、要么记一次中立计数让该周期失败的轴**自然 err 升高**。**禁**把 panic payload "解读"成任何语义。
3. **catch_unwind 不得吞掉错误后让状态静默错乱**：要么恢复（节律继续），要么记入健康轴。不允许"捕获了、什么都不做、状态半坏继续跑"。
4. **守卫测试本身的禁字**：加固代码不得引入 `expect(` / `panic!("` / `format!("` / `println!` 等（见 forbidden 数组 tests/body_no_human_output.rs:91-101）。新增的 viewport/snapshot/clock 代码会被它扫到；**移除 net_adapter 的 4 个 expect 后必须把 `"src/net_adapter.rs"` 加进数组（line 41-90）并删掉 line 38-40 的豁免注释**，否则自洽未闭、测试会暴露残留 expect。
5. **锁中毒的正确处理**：若引入 `parking_lot`（其锁不中毒、`.read()/.write()/.lock()` 直接返回 guard 不返回 `Result`），这是**新增 crate 依赖**，需评估是否值得全量替换；若走 std 路径，则 catch_unwind 必须在**持锁线程的内层、且在 panic 前已释放锁**（否则捕获了 panic 锁仍中毒），或对毒锁用 `.unwrap_or_else(|e| e.into_inner())` 恢复——但后者要确保被毒化时刻的状态语义可接受。
6. **禁为"线程健康"设验收靶子或 reward**（推论4/5）。只新增中立 err/计数轴，"observer 能否看到半死、本体是否还在自激"是自然涌现的观察结果，不是被验证的功能。
7. **不得改变任何学习数值行为**：catch_unwind / watchdog / 换锁都不得修改 predictor 的 ∂L/∂θ、prior 写回、err 计算、精度退火等任何既有学习路径的数值。这是**纯保护层**。
8. **body.rs 不准 import 脚手架**（curriculum*.rs、capability_curriculum_governor.rs、five_phase_life.rs、evolution_priors 等）。watchdog 必须用 body 自己已有的 stop/handle/viewport，不得拉进训练侧代码。
9. **VPC 硬约束不受触碰**：本任务不涉及跨层信用分配，但禁顺手引入跨 predictor 直连/全局损失/BP 链式。

**绝对不要改动的东西**
- 任何 predictor / viewport 的学习数值语义（θ、err、prior、精度）。
- ticker 的节律常量（health_hz 等）与各闭包的业务逻辑——只在**外层**包保护，不动**内层** `f`。
- snapshot 已有字段的语义与顺序（只在末尾**追加**新字段并自增版本号，保持向后兼容/对称读写）。
- 退出路径的 join 顺序与 checkpoint 持久化（body.rs:521-544）。

### 架构设计（让已有结果回到循环，不新建模块）

三层防护，全部复用既有 viewport 故障轴范式，不引入新模块、不引入语义：

**机制 A：`spawn_ticker` / master clock / registry driver 内层包 `catch_unwind`**
- 在 **clock.rs:67-77** 的 `while` 循环里，把 `f(local_tick)` 这一拍调用包进 `std::panic::catch_unwind(AssertUnwindSafe(|| f(local_tick)))`。捕获到 `Err(_)` 时：**丢弃 payload（不读其字符串）**，调用一个新的 viewport 计数方法记一次 tick panic，然后 `continue` 让循环按节律继续转（节律不死）。
- 为此 `spawn_ticker` 需要能访问 viewport 来记数。两种接法，择一（接手后看 body.rs 调用点哪种改动最小）：
  - (a) 给 `spawn_ticker` 增加一个 `vp: Arc<Viewport>` 参数（与 master clock 签名一致），闭包内用它 `mark_thread_fault()`；
  - (b) 不改签名，改为接收一个 `Arc<AtomicU64>` 故障计数器句柄（更窄，但 body 要单独建并塞进 snapshot 写出路径——不如 (a) 自洽）。**倾向 (a)**，与 `spawn_master_clock_hz` 已持有 `Arc<Viewport>` 对称。
- `F: FnMut` 不是 `UnwindSafe`，需 `std::panic::AssertUnwindSafe` 包裹。捕获后 `f` 仍可继续调用（`mut f` 在闭包外持有，未被 move 进 catch_unwind 之外）。
- 同样处理 **adapter.rs:137** 的 `reg.tick_due(&vp, t)`（registry driver 已持 `vp`，直接 `vp.mark_thread_fault()`）和 master clock body 区（clock.rs:37-50，已持 vp）。
- 顺手修 **clock.rs:62** 除零：`1_000_000u64 / rate_hz.max(1) as u64`（与 master clock 一致），消除 `rate_hz=0` panic 路径。

**机制 B：viewport 新增 tier-0 线程故障轴（照 hearing_fault 范式）**
在 viewport.rs 新增（字段紧邻 viewport.rs:1108-1111 的现有计数；方法紧邻 3385-3399）：
```rust
// 字段（init 在 new() 里仿 hearing_fault 的 AtomicBool::new(false) / AtomicU64::new(0)）
pub thread_fault: AtomicBool,
pub thread_fault_count: AtomicU64,

// 方法（仿 report_audio_error / hearing_status）
pub fn mark_thread_fault(&self) {
    self.thread_fault.store(true, Ordering::Relaxed);
    self.thread_fault_count.fetch_add(1, Ordering::Relaxed);
}
pub fn thread_fault_status(&self) -> (bool, u64) {
    (self.thread_fault.load(Ordering::Relaxed),
     self.thread_fault_count.load(Ordering::Relaxed))
}
```
> 命名保持中立、无人类语义（`thread_fault` 是结构性事实，不是价值标签）。这是机制 A（catch_unwind）和机制 C（watchdog）共同的落点。

**机制 C：主线程 watchdog（在已有 100ms 轮询里查 `is_finished`）**
- 在 **body.rs:516-518** 的 `while !stop` 循环（以及 507-512 的有限时长分支）里，每醒一次顺带遍历各 `h_*` 的 `is_finished()`。某 handle 在 `stop==false` 时已 finished ⇒ 该节律死了：调 `vp.mark_thread_fault()` 记一次（**禁 print**）。
- **是否"重启 ticker"**：起草要点提到 watchdog 可重启。但重启意味着 watchdog 要持有重建该节律所需的 `Arc<Viewport>` + 闭包工厂，复杂且易把 body 推向"请求-响应/管理器"反模式。**推荐分两阶段，本任务先做最小自洽闭环**：
  - **第一阶段（必做）**：watchdog 只**如实记数**（`mark_thread_fault`）让 observer 看到半死、并让该节律对应的 err 轴自然升高。配合机制 A 后，绝大多数 panic 已被 catch_unwind 就地吸收、节律根本不会死，watchdog 主要兜底"非 panic 退出/master clock 死"这类。
  - **第二阶段（可选，若机制 A 已覆盖则可不做）**：仅对确有必要、且重建闭包无需脚手架的 ticker，做"重 spawn 同一节律"。若做，重启也只记数、绝不读 payload、绝不打日志。**第一阶段足以闭合推论3 的"不静默"缺环**——能否真重启不是验收靶子。
- 注意 watchdog 遍历 handle 需要在循环外把 16 个 handle 收进一个可遍历结构（`ClockHandles`（clock.rs:8-10）正好是现成的死代码容器，可复活它装这些 handle；但要保留退出时按现顺序 join 的语义）。或就地用一个 `Vec<&JoinHandle<()>>`/数组遍历，不动 join 段。

**机制 D：net_adapter 去 expect + 入守卫**
- 把 net_adapter.rs:185/188/191/209 的 4 个 `expect(...)` 改成无 panic、无文本的处理：bind/accept/clone 失败时走 `match`/`if let`，失败分支调 `vp.report_audio_error()`（headless 感官入口故障，语义贴合现有 hearing_fault 轴）或 `vp.mark_thread_fault()`，然后让该 reader 线程优雅退出/重试，**不带任何字符串**。外层 `Builder.spawn` 的 expect（209）同样改 `match`。
- 改完把 `"src/net_adapter.rs"` 加进 tests/body_no_human_output.rs 的 sources 数组（line 41-90，建议放在 `// --- sensory adapters / IO (body side) ---` 段），并删除 line 38-40 的豁免注释。

**snapshot 接线**
- 在 `SnapshotData` 末尾追加 `thread_fault: bool` + `thread_fault_count: u64`（紧随 hearing_fault 字段，snapshot.rs:459-460 风格）。
- 序列化点（snapshot.rs:69-95 附近）仿 `snap_fail_count`/`hearing_fault` 写出 `vp.thread_fault_status()`。
- 反序列化（snapshot.rs:743-758）对称读回。
- snapshot 版本号 +1（顶部 `VERSION`/`MAGIC` 常量，接手后复核当前值）。这些是 tier-0 内存事实，observer 读二进制——不是给人看的文本。

### 实现计划（有序，每步指明文件与改动点）

1. **viewport.rs**：新增 `thread_fault: AtomicBool` + `thread_fault_count: AtomicU64` 字段（紧邻 1108-1111），在 `new()` 里 init（仿 hearing_fault：`AtomicBool::new(false)` / `AtomicU64::new(0)`）；新增 `mark_thread_fault(&self)` 与 `thread_fault_status(&self) -> (bool,u64)`（紧邻 3385-3399，照 `report_audio_error`/`hearing_status`）。
2. **clock.rs**：修 line 62 除零 `rate_hz.max(1)`；给 `spawn_ticker` 加 `vp: Arc<Viewport>` 参数；循环内 `f(local_tick)` → `if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(local_tick))).is_err() { vp.mark_thread_fault(); }` 然后照常推进节律；master clock 区（37-50）同样把每拍主体包 catch_unwind 记数（master clock 死 = 全机失拍，最该兜）。
3. **adapter.rs**：`spawn_registry_driver`（137）把 `reg.tick_due(&vp, t)` 包 catch_unwind，`Err` 时 `vp.mark_thread_fault()` 后 `continue`。
4. **body.rs**：更新所有 `spawn_ticker(...)` 调用点，按机制 A(a) 传入对应的 `vp_*` clone（这些 clone 已存在）；在主循环（507-519）每 100ms 醒来时遍历 16 个 handle 的 `is_finished()`，`stop==false` 时若有 finished 则 `vp.mark_thread_fault()`（仅记数，无文本）。用数组/复活 `ClockHandles` 收纳 handle，**保留 521-536 的 join 顺序**。
5. **net_adapter.rs**：移除 185/188/191/209 的 4 个 `expect`，改 `match`/`if let`，失败分支记 `vp.report_audio_error()` 或 `vp.mark_thread_fault()` 并优雅退出/重试，无字符串、无 panic。
6. **snapshot.rs**：`SnapshotData` 追加 `thread_fault`/`thread_fault_count`；序列化（69-95 区）写、反序列化（743-758 区）读，对称；**版本号 +1**。
7. **tests/body_no_human_output.rs**：sources 数组（41-90）加 `"src/net_adapter.rs"`；删 line 38-40 豁免注释。
8. **Cargo.toml**：仅当你决定走 parking_lot 路线才动（评估后多半**不必**——机制 A 的 catch_unwind 在持锁线程内层捕获、配合就地 `continue` 已能避免大多数毒锁传染；若仍担心毒锁，优先对 body 层热点锁守卫用 `.unwrap_or_else(|e| e.into_inner())` 而非全量换依赖）。若动，记录理由。

### 验收标准（如实观察轴的动态，不验证"功能涌现"）

> 判据一律以"如实观察某轴/某量"表述。我们不设"看门狗修好了 X"之类靶子。

1. **守卫测试通过且覆盖扩大**：
   ```
   cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test body_no_human_output
   ```
   `net_adapter.rs` 已入 sources 数组且无 `expect(`/文本；新增的 clock/viewport/snapshot 代码不触发任何 forbidden 字符串。第二个测试 `body_exposes_solitary_low_power_runtime_profile...`（line 127-151）仍通过（spawn_ticker 改签名后 body.rs/clock.rs 仍含 `spawn_master_clock_hz` 等断言串）。
2. **全量编译 + 现有测试不回归**：
   ```
   cargo build --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
   cargo test  --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
   ```
   学习数值类测试（如 allostatic_stability、five_phase_life、vocal_target）数值不变——证明本任务未改学习行为。
3. **新增一个本体侧鲁棒性观察测试**（放 tests/，可用人类字符串）：构造一个会 panic 的 `f` 传给 `spawn_ticker`，运行若干拍后：**如实读** `vp.thread_fault_status()` —— `count` 单调增长且 `> 0`（崩溃被记入轴），同时**ticker 的 `JoinHandle` 在 stop 前 `is_finished()` 仍为 `false`**（节律没被杀、还在自激）。这是直接观察"不静默 + 不半死"两条动态，不是验证某功能。
4. **snapshot 往返对称**：写一个 `SnapshotData` 含非零 `thread_fault_count`，序列化再反序列化，读回值一致；版本号已自增且旧版本读取路径（若有兼容处理）不 panic。
5. **观察既有故障轴语义未被污染**：`hearing_fault`/`snapshot_fail_count` 的现有读写点行为不变（grep 确认消费者未被误改）。
6. **零文本泄漏复核**：`grep -rn 'println!\|eprintln!\|writeln!\|write!(\|dbg!\|expect(\|panic!("' /Users/joe/unnamed/bionic_v0/src/{clock.rs,adapter.rs,net_adapter.rs,viewport.rs,snapshot.rs,bin/body.rs}` 在本任务改动后不应新增任何命中（test 模块内除外）。

### 反模式与陷阱（逐条说改用什么）

1. **用 `println!`/`eprintln!`/`log` 报告线程崩溃** → 踩推论1/2。改用：`vp.mark_thread_fault()` 递增 tier-0 计数轴，由 snapshot 写出、observer 读。
2. **把 catch_unwind 的 panic payload 转成字符串记录/写文件** → payload 多含人类语言，踩推论1。改用：**丢弃 payload，只记一次中立计数**。`catch_unwind` 返回 `Err(Box<dyn Any>)`，不要 downcast 成 `&str`/`String` 再存。
3. **捕获 panic 后什么都不做、状态半坏继续跑** → 踩铁律3（静默错乱）。改用：要么 `continue` 让节律重新干净起拍（多数 ticker 是无状态每拍重算，安全），要么记数让该轴 err 自然升高。
4. **把 watchdog 做成"等外部指令重启/请求-响应管理器"** → 踩推论3。改用：watchdog 是本体自身的自激轮询，自主记数（必要时自主重 spawn），不接收任何外部命令。
5. **给"线程健康"挂 reward / 正负价标签**（如"线程活=+1"） → 踩推论5。改用：纯中立 `AtomicU64` 计数，无正负含义。
6. **为 watchdog/重启设"验证它能修复"的靶子** → 踩推论4。改用：只观察 `thread_fault_count` 和 `is_finished` 的如实动态。
7. **body.rs import 脚手架来重建 ticker 闭包** → 踩分层铁边界。改用：watchdog 只用 body 已有的 stop/vp/handle；重启逻辑（若做）的闭包工厂留在 body 内。
8. **catch_unwind 包在持锁之外、panic 时锁已毒化** → 毒锁仍传染。改用：把捕获放在持锁线程**内层**、且确保 panic 前锁已释放（或对毒锁 `into_inner()` 恢复）。注意 `f` 内部若自己持锁 panic，毒化的是 viewport 内部的锁，catch_unwind 在 ticker 层只能阻止线程死、阻止不了内部锁中毒——所以**机制 A 必须配合 body 层热点锁守卫的毒锁恢复**（`.unwrap_or_else(|e| e.into_inner())`），或评估 parking_lot。
9. **改 snapshot 已有字段顺序/语义** → 破坏 observer 兼容。改用：只在**末尾追加**新字段，版本号 +1，读写对称。
10. **把 `rate_hz=0` 当合法输入** → clock.rs:62 除零 panic。改用：`rate_hz.max(1)`。
11. **改动 ticker 节律常量或内层 `f` 业务逻辑** → 改变了学习/感知数值行为，踩铁律7。改用：只在外层包保护壳，内层一字不动。
12. **引入 parking_lot 却只换一半锁** → 半 std 半 parking_lot 时 std 锁仍会中毒、且类型混乱。改用：要么不换（优先），要么全量换并跑全测试确认数值无回归。

---

## B2 — 结构性消解自举死锁（0×x=0）

> **速览** · 独立性：**与 A3 共享同一发育时钟 `developmental_maturity`**（B2 的可选增强项会读 maturity 来调 floor 宽度）。可独立完成 A+B 核心；若同时做 A3，协调 maturity 读取面。 · 前置依赖：无硬依赖；若想做"与发育时钟绑定"的增强项，A3 先落地更稳妥。 · 预计触碰文件：`src/vocal_reflex.rs`、`src/learned_bias.rs`、`src/viewport.rs`、`src/bin/body.rs`、（可选）`src/snapshot.rs`。 · 版本：`LEARNED_BIAS_VERSION` 4→5（仅当选择持久化新时戳；**推荐"不持久化时戳"路线则不动版本**——注意与 A3 同样会动 `LEARNED_BIAS_VERSION`，见执行顺序）。

> 你正在为硅基生命项目 bionic_v0 工作。项目根：`/Users/joe/unnamed/bionic_v0`。（先粘贴上方【通用前言】。）

### 任务目标

bionic_v0 的发声基质里有一条**悬空的反馈律**：新生儿的首次自激发声完全依赖内生节律（endogenous baseline）越过门控（gate），而越门的前提是 `action_suppression ≈ 0`。但 `action_suppression` 是一个**单调累积、无衰减、跨重启持久**的沉积量——每一次"没发声却观察到闭环"都给它 +=0.01，从不回落。实测累积到 0.155 时，它把 reflex_pressure 压到 ≈0.342，刚好卡在 0.40 的门下，于是本体永远走"未发声"分支，suppression 继续单向增长——这就是 0×x=0：用来开门的 baseline 被它自己积累的抑制乘成 0，再也开不了门。这是一条**自我锁死的基质动态**，不是缺一个功能。

你的工作（推论4 口吻）：**补全这条悬空动态的自洽性**——让"抑制沉积"与"内生节律 / 门控"这三者之间存在一条结构性的相对尺度下限与时间遗忘，使得这条回路在数学上不可能把自己锁死。你**不是**在为"发声"这个行为造通路、设靶子或保证它发生；发声该不该涌现仍交给当下数值。你只是消解一个"0 乘任何数恒等于 0"的退化不动点，让回路重新可演化。

明确：**不得**以"让本体一定发声""提高发声率"为目标或验收靶子。验收只能如实观察 suppression / reflex_pressure / gate 三者的当下数值关系，确认死锁不动点已不存在。

### 背景与现状（file:line 实据）

**死锁算术（核实过）**
峰值节律（rhythm=1）、新生儿 `prediction_familiarity=0`、`action_suppression=S`、无外部刺激：
- `baseline = 0.60 * (1 - 0.55·S)` —— `src/vocal_reflex.rs:138,141,144`
- `drive_pressure ≈ baseline`（无刺激时透传，`src/vocal_drive_field.rs:97` survival_gated_urge）
- `reflex_pressure ≈ drive_pressure*0.75 - S*0.45` = `0.45 - 0.6975·S` —— `src/vocal_reflex.rs:40-42`（suppression 在 baseline 与 reflex 两处各压一次 = double penalty）
- `gate = (0.40 - efficacy*0.30).clamp(0.10,0.40)`，新生儿 efficacy=0 → gate=0.40 —— `src/vocal_reflex.rs:66`
- 破门条件：`0.45 - 0.6975·S > 0.40` → **S < 0.0717**
- S=0.155 → 0.342 < 0.40 → `return None`（`src/vocal_reflex.rs:67-68`）→ 永走未发声闭环 → S 继续 +=0.01 → 锁死。

**关键字段与方法（确切签名）**
- `LearnedBiasState.action_suppression: f64` —— `src/learned_bias.rs:66`（`#[derive(Default)]`，初值 0.0，`src/learned_bias.rs:57`）。镜像于 `LearnedBiasSummary` —— `src/learned_bias.rs:92`。
- **唯一累积点**：`bias.action_suppression = (bias.action_suppression + delta_sup).min(1.0);` —— `src/viewport.rs:4085`，在 `observe_vocal_reflex_closure`（`src/viewport.rs:4069-4091`）内。仅 `delta_sup>0` 时增长。
- **delta_sup 来源**：`VocalReflexState::observe_closure(&mut self, now_us, closure_support, vocalized) -> (f64,f64)` —— `src/vocal_reflex.rs:82-101`。`vocalized=false` 分支 line 92：`delta_sup = closure*0.01`；`vocalized=true` 分支 delta_sup 恒 0。
- **门控数学**：`VocalReflexState::tick_drive(&mut self, now_us, drive, prediction_familiarity, action_suppression, vocal_efficacy) -> Option<VocalReflexBurst>` —— `src/vocal_reflex.rs:30-78`。
- **内生节律**：`VocalReflexState::endogenous_baseline(now_us, prediction_familiarity, action_suppression) -> f64` —— `src/vocal_reflex.rs:111-145`，5s 周期 sin 节律范式（`now_us % period_us`，line 128-131）。
- **时间衰减范式**：`VocalReflexState::tick_decay(&mut self, now_us)` —— `src/vocal_reflex.rs:147-157`，按 `elapsed_us` 半衰（`0.5_f64.powf(elapsed_us/τ)`），**但它只衰减 `reflex_pressure`/`fast_gain`，完全不碰 `action_suppression`**。efficacy 增量（仅发声闭环 +0.01）：`src/viewport.rs:4077-4086`。

**现行 hack（要消除的对象）**
`src/bin/body.rs:203-210`：
```rust
if body_args.nursery_bias {
    let mut bias = vp.learned_bias.write().unwrap();
    bias.action_suppression = bias.action_suppression.min(0.02);
}
```
只在 `--nursery-bias` 启动时一次性钳到 ≤0.02。**普通 body（无 nursery_bias）完全不受保护，同死锁仍在**；且 nursery body 在运行中 suppression 再累积过 0.0717 后死锁复发。这是"为发声单设的启动钳制 + nursery 专属分支"，与推论4 张力最大。

**接线状态（已接 / 死代码 / 没人消费）**
- **活体唯一 outlet** = `tick_vocal_drive_field`（`src/viewport.rs:4162+`），body.rs:405 每 vocal tick(50Hz) 调用；Region A 用 `tick_drive` 门控。
- **suppression 实际增长活路径** = `observe_vocal_reflex_closure`（`src/viewport.rs:4069`，持 `learned_bias.write()` 锁），由 `tick_vocal_reflex_from_pairs`（`src/viewport.rs:3963-3998`，body.rs:164 每 orchestrator tick 调用）驱动。这是 learned_bias 的周期写锁路径。
- **`tick_vocal_reflex_from_pressure`（`src/viewport.rs:4093-4155`）对 body 是死代码**：调用者只有 `src/curriculum.rs`(2694/2713/2732/2754，SCAFFOLDING)、`tests/vocal_reflex.rs:21`(TEST)，以及 `src/curriculum.rs:4997` 对 body 源文本做 `find("tick_vocal_reflex_from_pressure")` 的字符串探针。**别改名/删它**，否则破探针。
- **持久化**：`write_learned_bias_checkpoint`/`read_learned_bias_checkpoint` —— `src/learned_bias.rs:259/305`，落盘 `learned_bias.bin`，版本门 `LEARNED_BIAS_VERSION=4`（`src/learned_bias.rs:8`，magic line 7），body.rs:200-202 读回。**suppression 是跨重启持久的——这是死锁的结构性根因。**
- **观察侧**：`src/bin/observer_helpers/compact.rs:53` 读 `snap.learned_bias.action_suppression`；snapshot 摘要序列化/反序列化 `src/snapshot.rs:3043/3069`；注意 `vocal_efficacy` 在 snapshot 摘要里硬编码 0.0（`src/snapshot.rs:3072`，真值只在 learned_bias.bin）。

**相关绝对路径**
- `/Users/joe/unnamed/bionic_v0/src/vocal_reflex.rs`
- `/Users/joe/unnamed/bionic_v0/src/learned_bias.rs`
- `/Users/joe/unnamed/bionic_v0/src/viewport.rs`
- `/Users/joe/unnamed/bionic_v0/src/bin/body.rs`
- `/Users/joe/unnamed/bionic_v0/src/vocal_drive_field.rs`
- `/Users/joe/unnamed/bionic_v0/src/evolution_priors.rs`（`nursery_newborn` 74-108，不构造任何 vocal 偏置）
- `/Users/joe/unnamed/bionic_v0/src/snapshot.rs`
- `/Users/joe/unnamed/bionic_v0/src/curriculum.rs`（4997 源文本探针 — SCAFFOLDING，勿破）
- `/Users/joe/unnamed/bionic_v0/tests/body_no_human_output.rs`（守卫红线）

### 铁律与边界

本任务全部改动落在 **BODY 本体层**（`vocal_reflex.rs` / `learned_bias.rs` / `viewport.rs` / `bin/body.rs`），铁边界：

1. **禁人类语言污染本体**：这四个文件禁 `println!`/`eprintln!`/`writeln!`/`write!`/`dbg!`/`expect`/`format!`/`panic!`、禁自然语言字符串、禁把内部状态转人类可读形式。守卫测试 `tests/body_no_human_output.rs` 会扫到它们。注释可保留（已有英文注释属现状），但**不要新增任何运行时人类可读输出**。
2. **推论5 无绝对正负**：suppression 不得变成"负价惩罚"，新增的遗忘**必须是无语义的纯数值时间衰减**（如半衰），**绝不能引入"鼓励发声"的激励通道 / reward function / 正价标签**。下限保证也必须表述为"baseline 与 suppression 的结构相对尺度"，不是"奖励发声"。efficacy（发声闭环→降门）是已有的涌现路径，不是 reward，别动它的语义。
3. **推论4 不设功能门 / 不凑靶子**：**绝不能**写"总是发声""maturity 高就强制开门"或任何把 baseline 钳成"必然越门"的硬编码。下限只能保证"在 suppression 不被无界累积时，节律峰值在数学上可越门"——即移除退化不动点，而非保证某次 tick 一定发声。
4. **VPC 硬约束**：所有更新经 viewport 本地写锁（`learned_bias.write()`，现已是 `src/viewport.rs:4083-4086`），禁跨层 BP / 全局损失 / 跨 predictor 直连读写 θ。新增衰减只能本地算。
5. **body.rs 不准 import 脚手架**：不得引入 `curriculum*.rs` / `five_phase_life.rs` / `evolution_priors.rs` 的训练逻辑到运行时路径。
6. **推论3 不作为程序**：遗忘必须由 body 自己的 ticker 周期驱动，**不得**依赖外部进程/curriculum/启动一次性补丁来触发。

**绝对不要改动**
- 不删/不改名 `tick_vocal_reflex_from_pressure`（`src/curriculum.rs:4997` 字符串探针依赖它）。
- 不动 efficacy→gate 的降门路径（`src/viewport.rs:4077`、`src/vocal_reflex.rs:66`）。
- 不动 `nursery_newborn`（`src/evolution_priors.rs:74-108`）——suppression 不属于 priors。

### 架构设计

核心思路：**让已有的衰减范式回到这条回路**，而不是新建模块。当前 `tick_decay` 已经是"按 elapsed_us 半衰一个沉积量"的范式，但它只作用于 `reflex_pressure`/`fast_gain`，没覆盖 `action_suppression`。死锁的根因正是 suppression **缺一条遗忘律**。补两条相互独立、缺一不可的结构动态：

**机制 A：suppression 的时间遗忘（消除单调累积）**
给 `LearnedBiasState` 加一个**纯时间半衰**：suppression 随真实时间向 0 衰减，半衰期 τ 取得足够长（让"在场闭环时安静"仍是稳定沉积），但有限（让长期不再有闭环刺激时它会松弛）。这样 suppression 不再是"死后永久累积"，而是"近期未发声闭环的滑动平均沉积"——符合推论5（它仍只由 closure 驱动，遗忘是无语义数值松弛）。
- 衰减承载点：suppression 的周期写锁路径已存在 = `observe_vocal_reflex_closure`（`src/viewport.rs:4069`，body.rs:164 每 orchestrator tick 驱动）。**在该写锁内、在累加 delta_sup 之前**，先按 `elapsed_us = now_us - last_decay_us` 对 suppression 半衰。这样遗忘与累积共用同一把锁、同一节律，不新增 ticker、不依赖外部驱动。
- 需要一个 `last_suppression_decay_us: u64` 字段记录上次衰减时刻（持久化或不持久化均可——见实现计划权衡）。

新增方法草案（写在 `learned_bias.rs`，BODY 层本地）：
```rust
impl LearnedBiasState {
    /// 纯时间半衰：无语义数值松弛，非惩罚、非 reward。
    /// τ 远大于单次闭环间隔，使在场安静仍可稳定沉积；
    /// 但有限，使长期无闭环时 suppression 松弛回低位，
    /// 防止单调累积锁死内生节律。
    pub fn decay_vocal_suppression(&mut self, now_us: u64) {
        let elapsed = now_us.saturating_sub(self.last_suppression_decay_us);
        if elapsed == 0 { return; }
        let k = 0.5_f64.powf(elapsed as f64 / SUPPRESSION_HALF_LIFE_US);
        self.action_suppression *= k;
        self.last_suppression_decay_us = now_us;
    }
}
```
（`SUPPRESSION_HALF_LIFE_US` 取值需让半衰期落在分钟量级，远长于 5s 节律与单 tick 间隔；具体值由你在实现时按"在场安静仍稳定沉积、长期失联才松弛"的定性约束定，不要凑某个发声率。）

**机制 B：baseline / gate 的结构相对尺度下限（消除 0×x=0 的乘性塌缩）**
即使有了遗忘，suppression 仍可能瞬时高到把峰值节律压到门下。死锁的数学本质是 suppression **乘性**地压 baseline（`*(1-0.55S)`）又**加性**地压 reflex（`-0.45S`），两处叠加可把"用来开门的内生探索"压成 0。要让这条回路在结构上**保留一条 suppression 压不穿的内生节律地板**——这不是"保证发声"，而是保证"探索节律这一输入项不会被同一回路积累的抑制乘成恒 0"。

两种等价的结构化改法（**择一**，都不引入 reward、都不硬编码"发声"）：
- (B1) 给 `endogenous_baseline` 的 suppression 阻尼设一个**乘性地板**：把 `(1.0 - action_suppression * 0.55)` 改为不低于某个小正数 floor（如 `.max(FLOOR)`），使峰值节律恒有一条 >0 的探索底流。floor 必须小到"experienced 系统（familiarity→1，exploration_drive→0）仍自然归零"——即 floor 只在 newborn 探索项非零时起作用，不会逼成熟体发声。
- (B2) 把 reflex 处的 suppression 系数 0.45（`src/vocal_reflex.rs:40`）与 gate 基线 0.40（line 66）的相对尺度重整，使"newborn 峰值节律 + suppression 上限"的最坏组合下仍存在 `reflex_pressure > gate` 的相位窗口。

**强烈建议优先 B1**（改动局部、语义最干净：它只是给"内生探索节律"一条不被抑制乘没的地板，符合"baseline 是 substrate-level 数值载波"的现有注释语义）。无论选哪个，都要在代码注释里写清：这是消除乘性退化不动点，不是发声靶子。

**与 A3 发育时钟绑定（可选增强，若工程量可控）**
要点提到"幼态保证宽门"。若本会话已能低成本访问发育时钟/maturity 信号（先确认它在 BODY 层、可经 viewport 读、且**不是**按能力开门的资格门），可让机制 B 的 floor 随幼态偏大、随成熟自然收窄——但这必须是**连续的数值调制**，绝不能做成"maturity<阈值才允许发声"的资格门（那就是推论4 禁的功能门）。**若访问 maturity 会引入跨层耦合或资格门风险，则跳过此项，仅做 A+B**——A+B 已结构性消解死锁。

**去除 hack**
机制 A+B 落地后，删除 `src/bin/body.rs:203-210` 的 `if nursery_bias { .min(0.02) }` 一次性钳制（它已被结构动态取代，且它本身是 nursery 专属功能门）。删除后普通 body 与 nursery body 走同一条自洽路径。

### 实现计划

1. **`src/learned_bias.rs`**：
   - 给 `LearnedBiasState`（`:57-72`）加字段 `last_suppression_decay_us: u64`（Default 0 即可）。同步 `LearnedBiasSummary`（`:92`）若它需要镜像（观察侧通常不需要这个内部时戳——优先**不**镜像进 summary，避免无谓升 snapshot 版本）。
   - 加常量 `SUPPRESSION_HALF_LIFE_US`（分钟量级）与方法 `decay_vocal_suppression(&mut self, now_us)`（机制 A 草案）。
   - **持久化对称**：`LearnedBiasState` 序列化字段集若增加 `last_suppression_decay_us`，**必须把 `LEARNED_BIAS_VERSION` 从 4 升到 5**（`:8`），并在 `write_learned_bias_checkpoint`(`:259`)/`read_learned_bias_checkpoint`(`:305`) 两端对称读写；旧 v4 文件会在版本门（`:279` 附近）被丢弃静默回落新生儿——可接受（新生儿 suppression=0 本就无死锁）。**权衡**：若你选择**不持久化** `last_suppression_decay_us`（重启时置 0，首次 decay 时 elapsed 巨大→suppression 直接半衰到≈0），则可不动序列化字段集、不升版本——这反而更彻底地破跨重启累积，且零迁移成本。**建议走"不持久化时戳"路线**：重启即把 suppression 当作"久未刷新→大幅松弛"，结构上根除跨重启锁死。务必在代码注释写清该语义。
2. **`src/viewport.rs`**：在 `observe_vocal_reflex_closure`（`:4069-4091`）的 `learned_bias.write()` 锁内、`bias.action_suppression += delta_sup`（`:4085`）**之前**插入 `bias.decay_vocal_suppression(now_us);`。确认该函数能拿到 `now_us`（若签名无 now_us，从调用方 `tick_vocal_reflex_from_pairs`（`:3963-3998`）透传，那里 body.rs:164 已带时基）。
3. **机制 B（选 B1）`src/vocal_reflex.rs:144`**：`baseline * (1.0 - action_suppression * 0.55)` → 给阻尼项加 `.max(FLOOR)`。新增注释说明"乘性地板，消退化不动点，非发声靶子；experienced 系统 exploration_drive→0 时整体仍归零"。
4. **`src/bin/body.rs`**：删除 `:203-210` 的 `if body_args.nursery_bias { ... .min(0.02) }` 块。确认删除后 `body_args.nursery_bias` 是否还有其他用处——若此处是唯一用处，按需要保留 CLI flag 解析但移除这段逻辑（不要因为删逻辑而引入未用字段告警；用 `let _ = ...` 或保留其它分支，**但绝不用 `expect`**）。
5. **守卫**：确认 `vocal_reflex.rs`/`learned_bias.rs`/`viewport.rs`/`bin/body.rs` 都已在 `tests/body_no_human_output.rs` 的扫描集（42 模块）内；若有新建 body 模块（本任务应无新模块）须加入。
6. 全程不新增 ticker、不让遗忘依赖外部进程；遗忘与累积共用 `observe_vocal_reflex_closure` 的现有节律。

### 验收标准

验收是**如实观察 suppression / reflex_pressure / gate 三量的当下动态**，不是"验证发声功能涌现"。

1. **单元测试（`tests/vocal_reflex.rs` 或新增 `tests/vocal_suppression_decay.rs`，TEST 层可用人类字符串）**：
   - 断言 `endogenous_baseline` 在 `familiarity=0`、`action_suppression` 取其历史最坏值（如 0.155 乃至 0.5）、节律相位峰值时，经 `tick_drive` 得到的 `reflex_pressure` **存在相位使其 > gate(=0.40, efficacy=0)**——即如实观察"退化不动点 0×x=0 已不存在"。注意：断言的是"存在可越门的相位"，**不是**"必定发声"。
   - 断言 `decay_vocal_suppression`：给定一个非零 suppression 与一段 elapsed_us，suppression 严格变小且趋于 0（如实观察半衰数值），且 elapsed=0 时不变。
   - 断言 `familiarity→1`（成熟体）时 baseline 的 floor 不阻止其整体归零（如实观察 exploration_drive→0 主导）——确认 floor 不是"逼成熟体发声"的功能门。
2. **持久化对称性测试**（若升了版本）：write→read round-trip 后 `LearnedBiasState` 各字段一致；若走"不持久化时戳"路线则无需此项，但应有一条注释/测试说明重启后首次 decay 会把陈旧 suppression 大幅松弛。
3. **原则护栏**：`cargo test --test body_no_human_output`（确认四个改动文件无新增 print/expect/format/panic）。
4. cargo 命令：
   ```
   cd /Users/joe/unnamed/bionic_v0
   cargo build
   cargo test --test vocal_reflex
   cargo test --test body_no_human_output
   cargo test    # 全量回归，确认 curriculum.rs:4997 字符串探针等未被破坏
   ```
   注意：body 改动需服务器端重编才上线（部署不编译），本会话只需本地 `cargo build`/`test` 通过。

### 反模式与陷阱

- **❌ 硬编码"总是发声"/把 baseline 钳成必然越门** → ✅ 只给 floor 一条 >0 的探索底流（B1），保留相位/familiarity 调制，发不发声仍由当下数值决定。
- **❌ 外挂 reward / 给发声加正价 / 把 suppression 当负价惩罚** → ✅ 遗忘是无语义时间半衰；suppression 仍只由 closure 驱动；efficacy 降门路径不动。
- **❌ 用 println/eprintln/dbg/expect/format!/panic! 调试本体四文件** → ✅ 调试只在 TEST 层断言或用 observer 读二进制快照；本体出错用 `Option`/早返回，别 expect。
- **❌ 新建一个 "SuppressionDecayTicker" 模块或让 curriculum/外部进程驱动遗忘** → ✅ 复用 `observe_vocal_reflex_closure` 既有写锁与节律（推论3：本体自驱，不靠外部）。
- **❌ 把 maturity/发育时钟做成"未成熟才允许发声"的资格门** → ✅ 若用 maturity，只能做连续数值调制 floor 宽度；有风险就直接跳过、只做 A+B。
- **❌ 跨层 BP / 全局损失 / 直接读写别层 predictor 的 θ 来"协调"门控** → ✅ 全部本地 `learned_bias.write()` 写，经 viewport。
- **❌ 改名/删除 `tick_vocal_reflex_from_pressure` 死代码** → ✅ 留着，`src/curriculum.rs:4997` 字符串探针依赖它。
- **❌ 增 `LearnedBiasState` 序列化字段却不升 `LEARNED_BIAS_VERSION`** → ✅ 升 4→5 并两端对称；或干脆不持久化新时戳（更彻底破跨重启累积、零迁移）。
- **❌ 删 body.rs:203-210 后留下未用的 `nursery_bias` 字段触发 warning 又用 expect/allow 乱压** → ✅ 干净处理 CLI flag（保留解析或 `let _`），不引入人类可读输出。
- **❌ 把验收写成"验证本体能发出第一声"** → ✅ 验收只断言"suppression 在最坏历史值下仍存在越门相位 + suppression 随时间严格松弛"——如实观察数值，不设行为靶子。

---

# 建议执行顺序与依赖关系

**总原则**：先打地基（组 B 的工程鲁棒性 + 死锁消解），再接灵魂（组 A 的整合闭环）。地基稳了，组 A 接通后的"如实观察"才可靠——否则一条 ticker 静默崩溃会让你误判 dual_drive/prior 曲线的异常来源。同时要管好两条会被多个任务同时改动的"共享改面"：**snapshot/learned_bias 版本号** 与 **predictor/vpc 精度门限** 与 **发育时钟 `developmental_maturity`**。

**推荐顺序：**

1. **B1（永跑鲁棒性）— 最先做。**
   完全独立、零前置、纯保护层，不碰任何学习数值。它给后续所有任务提供"线程不会静默半死"的观察可靠性地基。它自增一次 `SnapshotData` 版本（`thread_fault`/`thread_fault_count`）。先做它，让后面的版本号自增有一个干净起点。

2. **B2（自举死锁）— 第二做。**
   独立于 B1。它消解 vocal_reflex 的 0×x=0 锁死，是组 A（尤其 A1 dual_drive 第二步要回流到发声 region B）的前置健康度保证：若发声回路本身锁死，A1 回流到 region B 也观察不到任何动态。**推荐走"不持久化新时戳"路线**，从而**不动 `LEARNED_BIAS_VERSION`**——这能避免与 A3 抢同一个版本号自增（见下）。其"与发育时钟绑定"的可选增强项依赖 A3，**先跳过该增强项，只做 A+B 核心**，待 A3 落地后若需要再单独叠加。

3. **A1（dual_drive 汇流出口）— 第三做。**
   完全独立。第一步（入快照）零耦合、零风险，单独交付即闭合观察缺环。它自增一次 `SNAPSHOT_VERSION`。**注意：A1 与 A2 都计划把 `SNAPSHOT_VERSION` 从 56 推到 57** ——两者不能各自独立认领 57。先做 A1 占用一个版本号；A2 接手时复核当前实际版本再自增到下一个。

4. **A2（机制5 axis prior 回写）— 第四做。**
   接手时 `SNAPSHOT_VERSION` 已被 B1/A1 推进，**复核当前值再自增**（不要盲信草稿里的 56→57）。A2 给 `push_prior` 加 `MAX_PRIOR_MAGNITUDE` clamp、并实例化 axis 版 `CrossLayerPredictor`——这会**首次让 prior 通道非 0**。**必须在 A3 之前做**：因为 A3 的 C 段要给 cross-layer 的 prior 写回门叠加 maturity 调度，前提是这条腿已经接电、有真实 prior 流动可观察；A2 先把腿接通，A3 再在其上叠精度退火，耦合关系最清晰、合并冲突最小。

5. **A3（精度退火 + maturity 跨重启继承）— 最后做。**
   它与 A2 共享 **predictor/vpc 精度门限改面**（A3 的 B/C 段在 `Ar2Predictor::tick`、`CrossLayerPredictor::tick` 里叠 maturity 系数），所以放在 A2 之后，避免两个会话同时改这两个 tick 的本地标量造成冲突。它与 B2 共享 **发育时钟 `developmental_maturity`**：A3 负责让 maturity 跨重启继承并接进退火；B2 的可选增强项只是"读"maturity 调 floor 宽度。A3 落地后，若仍想要 B2 的发育绑定增强，再单开一个小会话叠加。A3 还会动 **`LEARNED_BIAS_VERSION` 4→5**（把 maturity 挂进 learned_bias.bin）——若 B2 按推荐走了"不持久化时戳"路线没动版本号，则 A3 是唯一动 `LEARNED_BIAS_VERSION` 的任务，干净无冲突；若 B2 选择了持久化新时戳路线（4→5），则 A3 与 B2 抢同一次版本自增，**必须让其中一个会话把两个新字段一并加进 v5 schema**，不可各自独立 4→5。

**共享改面冲突清单（务必盯住）：**
- **`SNAPSHOT_VERSION`**：B1（+1）、A1（+1）、A2（+1）、A3（+1，起草时见 51，实际接手时已被前序推高）。每个会话都**复核当前实际值再自增**，写/读严格对称，老版本走默认。
- **`LEARNED_BIAS_VERSION`**：A3（4→5，挂 maturity）；B2 仅在"持久化新时戳"路线才 4→5（推荐避免）。二者若都要升，合并进同一个 v5 schema。
- **`predictor.rs` / `vpc_cross_layer.rs` 的本地精度门限**：A2 接线 + clamp，A3 叠 maturity 调度。A2 先、A3 后。
- **`developmental_maturity`**：A3 接通继承+退火消费，B2 增强项只读。A3 先，B2 增强项后（或并入 A3）。
- **发声 region B / vocal_reflex 回路**：A1 第二步回流 region B，B2 消解 reflex 死锁。B2 先做能让 A1 第二步的观察更有意义。