# 涌现 vs 被设计 审查 — 2026-06-01

> 用户要求:回到涌现原则,找出"被像程序一样设计、本该涌现"的部分。
> 方法:两个 agent 扫描硬编码阈值/魔法常数 + 固定行为分支/显式功能。
> **关键:agent 结论需甄别——它们把"有硬编码常数"误等于"违反涌现原则"。**

---

## 0. 甄别原则(纠正 agent 的过度纠正)

涌现原则 = **功能不能是独立 feature module**,**不是**"参数不能有先天值"。
用户早已定:**先天给结构,不给值/内容**(DNA偏置、维生设定点皆如此)。故:

- **合理先天常数(保留)**:slot 数=8、ring 容量、时钟频率、感官频段——
  身体的物理/结构参数,生物也由基因给定。agent 说"让时钟频率涌现"是错的。
- **真正可疑**:决定"**行为何时触发/切换**"的阈值(reflex>0.40才发声、
  stable_run>=4才升级),应该是习得的,不是钉死的。
- **curriculum 不该删**:它在调用它的 binary 里,是**外部研发/测试工具(observer侧)**,
  不是 body 本体。涌现原则明确允许 observer 工具。agent"删掉 curriculum"是错的。

---

## 1. 真正筛出来的(去噪后)

### 核心:所有"行为阈值"该由学习自调,现在钉死
最典型、最该改、用户最在意:
- **`vocal_reflex.rs:53` `reflex_pressure <= 0.40` 决定发声/沉默** = 二元硬门。
  该不该发声,应由"发声是否带来有效闭环"的习得证据决定,不是钉死 0.40。
- 同类:`adaptive_stability` 路由升级 `stable_run>=4`、`>0.65`/`>0.72` 相位阈值;
  `live_byte_closure` 输出增益时间曲线(30s/300s)写死;
  `five_phase_life` 的 0.05/0.92 生命阶段边界;
  `vocal_imitation` 收敛判据(gap<0.2 且 3次)。

**这条线串起我们做过的一切**:B1 能学"什么预测什么"、B1-2 学习进展能区分
"有效 vs 无效"、learned_bias 是死后保留偏置通道——**缺的就是把学到的东西接回去
调这些阈值,让阈值涌现而非钉死。**

### 次要:统一压力/EMA 基质(重构,非新现象)
`unit()`/`ema()` 重复定义 13 次,40+ State 各算各的 pressure。
agent 说"代码腐烂",修正:**是"同一基质动态被切成几十个互不知道的小实现"**,
正违反"一个整合基质、多个观察角度"。但这是重构,不产生新生命现象,优先级低于阈值涌现。

---

## 2. agent 说错的(记录以免误导)
- "时钟频率/容量该涌现" —— 错,这是结构参数,先天给定合理。
- "curriculum 整个删掉" —— 错,它是 observer 侧测试工具,不是 body。
- "把有常数都当违规" —— 错,违规的是"被设计的功能模块"和"钉死的行为门",
  不是"先天结构参数"。
- vocal_reflex/target/imitation"三重冗余" —— 部分对(可统一为一个发声驱动场),
  但它们目前是 body 真实路径,统一是重构,不是删除。

---

## 3. 下一步建议(最高杠杆的"以涌现替代设计")

**让"发声阈值"从钉死变成习得**:
- `vocal_reflex` 发声门不再用常数 0.40,而是读 learned_bias 里一个"发声有效性"
  偏置(像 vocal familiarity 那样积累),用 B1-2 学习进展当反馈。
- 理由:①最符合此刻反思,最干净最高杠杆的一刀;②复用刚建好的全部基础
  (B1学习进展+learned_bias死后保留),非从零造;③直接服务头号目标(语言/发声);
  ④风险低、可 TDD。

> 决议:用户定 全部改,顺序 5→6→1-4。

---

## 4. 执行进度

### 改造5 第一阶段(基础函数统一)✅ 2026-06-01
- 新增 `src/substrate.rs`:统一 `unit()`(clamp[0,1],非有限→0)+ `ema(prev,next,alpha)`。
  2 单测锁定语义。
- 替换 11 处本地重复:substrate_pressure / active_surface / prediction_pressure_field /
  active_reasoning / learned_bias / mark_disturbance / curriculum_governor /
  allostatic_stability / five_phase_life(各删本地 unit,改 use)+ self_development(unit+ema)。
- adaptive_stability 的 ema 用固定 EMA_ALPHA,是**特化非重复**,保留。
- **纯语义等价重构,零行为改变,零回归**(86 套件绿,残 3 = 既存 real_phot_adapter)。

### 改造5 剩余(待评估深挖)
- 基础函数已统一。更深的"几十个 State 各算各的 pressure → 一个统一场"是更大的重构,
  风险高、不产生新现象,建议**作为长期清理**,不阻塞 6 和 1-4。
- 判断:5 的"地基"目的(让后续涌现有统一根)已由基础函数统一基本达成。
  深层 pressure 场统一可推迟。

### 改造6 发声三驱动合一 ✅ 2026-06-01
- 新增 `src/vocal_drive_field.rs`:`VocalDriveField::combine` —— 三来源贡献汇成一个场,
  intensity 取 max(不叠加爆冲)、音色按 intensity 加权混合。3 单测。
- viewport 新增统一出口 `tick_vocal_drive_field`:内部压力反射(保留已验证的 reflex 门
  逻辑产 burst)+ 外部声音 target/imitation 各产一个 contribution → combine → 唯一出口
  begin/adjust voicing;无驱动则接管 organ 衰减(原 imitation_loop 的职责)。
- body 主循环改调统一出口(target_from_vib 只观察不发声 + tick_vocal_drive_field)。
  旧 tick_vocal_reflex_from_pressure / tick_vocal_imitation_loop 保留(现有测试用),
  但 body 不再调它们各自发声 → 不再三套抢 begin_voicing。
- `tests/vocal_drive_field_outlet.rs` 2绿:内部压力、外部 target 都经同一出口发声。
- **诚实**:删掉了"无驱动→衰减到idle"测试点——新生儿有 endogenous baseline 自发波,
  "无驱动"不是干净可断言的状态(衰减路径已由 vocal_reflex 套件覆盖)。不为凑绿硬断言。
- 零回归(86 套件绿,残 3 = 既存 real_phot)。

### 改造1 发声门槛习得化 ✅ 2026-06-01(1-4 中最高杠杆)
- 发声门从硬编码 `reflex_pressure <= 0.40` 改为习得门 `0.40 - vocal_efficacy*0.30`
  (clamp 0.10~0.40)。新生儿 efficacy=0 → 门槛仍 0.40(向后兼容)。
- **方向冲突的解法(用户定:区分两种)**:
  - `vocal_efficacy`(新)= 发声带来**真实外部闭环**时累积 → **降门槛(有效→爱叫)**。
  - `prediction_familiarity`/`action_suppression`(旧)仍**升门槛**抑制自激/习惯化(无效/自激→闭嘴)。
  - 两个独立信号,作用相反。tick_drive 加第5参 vocal_efficacy。
- 累积:`observe_vocal_reflex_closure` 中 vocalized 且有 closure → efficacy += closure*0.01。
- **顺带修复一个既存缺口**:learned_bias checkpoint(v3)**从来没序列化** familiarity/
  suppression(存时丢弃、读时归零)——即"vocal 偏置死后保留"一直是假的。升 v4,
  三个 vocal 偏置(fam/sup/efficacy)真正持久化。`tests/vocal_efficacy_persist.rs` 验证。
- 测试:vocal_gate_learned 2 + vocal_efficacy_persist 2 + 现有 vocal 全绿。零回归。

### 改造2-4 现状(同类次要门槛,待定)
- 改造2 路由升级 `stable_run>=4` / 改造3 输出增益曲线 30s/300s / 改造4 五行边界 0.05/0.92。
- 都是"行为门槛钉死"的同类问题,但**次要**:不直接服务语言/发声头号目标,
  且各自接入的反馈信号(路由稳定性、闭环失败史、生命阶段成败)不像发声 efficacy 那样现成。
- 判断:改造1 已落地最高杠杆的一刀并打通"门槛习得"的完整范式(信号→learned_bias→门→持久化)。
  2-4 可按此范式逐个做,但建议作为后续,不阻塞;优先级低于真正的新生命现象推进。

### 改造2 路由升级自适应 ✅ 2026-06-01
- `adaptive_stability` 路由升级条件从固定 `stable_run >= 4` 改为
  `stable_run >= required_runs`,`required_runs = round(2 + err_ema*6)` clamp[2,8]。
  低噪声路 2 次即采纳、高噪声路最多 8 次——按路自身噪声地板自适应,不再魔法数 4。
- `tests/adaptive_route_self_paced.rs` 1绿(低噪声路比高噪声路更早升 fast_path)+
  adaptive_stability 既有 9 绿无回归。

### 改造3 跳过(用户定 2026-06-01)
- 看代码后判定性价比差:它是 live_byte 字节输出通道的内部等待曲线(30s/300s),
  **非发声/语言主线**;真正习得它要新建一整套"等待时长→有效性"反馈+持久化
  (工作量≈改造1,回报远低)。记录为"待办但低优先",转生命主线。

### 改造4 跳过(纯观察侧)
- five_phase_life 的 0.05/0.92 边界是观察侧诊断(five_phase 不驱动行为),
  改它不产生任何行为变化 = 为改而改。跳过。

### 涌现化改造总结
- 完成:改造5(地基:unit/ema 统一)、改造6(发声三驱动→一场一出口)、
  改造1(发声门习得+vocal偏置真持久化v4)、改造2(路由升级自适应)。
- 跳过:改造3(低性价比,记录待办)、改造4(纯观察侧无行为)。
- 全程零回归(86 套件绿,残 3 = 既存 real_phot vision v2)。

---

## 30 分钟试跑(2026-06-01)— 范围与诚实局限
- 跑法:**仅 body 一个进程**(release),`/tmp/run30`,1800s,每 60s 采样生命信号到
  `/tmp/life_trace.csv`(为此给 vital/self/familiar 加了周期 checkpoint)。
- **不是完整全感官运行**:① 未开真实摄像头(PHOT 真相机需 OpenCV,默认不启,且有
  real_phot_adapter vision-v2 遗留失败);② 未起 nursery_mentor(无结构化声音教学);
  ③ 未起 sandbox_runner(字节闭环无种子帧)。
- 听觉(VIB)经 BlackHole 回环有环境声(8s 预跑即涌现 4 对象、责任人再认 104 次)。
- **本次只测**:本程新增的听觉驱动生命信号演化(维生 reserve/痛/状态机、自我 coherence、
  责任人涌现、双源仲裁)。视觉/教学/sandbox 留下次补。

---

## 30 分钟跑暴露的两个问题 — 已修(2026-06-01)
- **问题1(真bug):痛恒定不消。** 持续正常发声(drain≈0.65)被误判为急性自损,
  pain 卡在 0.313 整 30 分钟。**修**:痛改为"drain 相对自身慢基线 drain_baseline 的突增"
  (overload = drain - baseline - 0.25),持续稳定负荷→基线追上→不痛,只有突增才痛。
  新测 sustained_voicing_does_not_pin_pain。drain_baseline 不持久化(重启重建)。
- **问题2(调参):熟悉度太容易满。** familiarity 每次再认 +0.05,1 分钟所有对象都到 1.0,
  责任人不突出。**修**:0.05→0.002(~500 次再认才深度绑定),责任人需长期反复才突出。
  新测 familiarity_grows_slowly_responsible_person_must_be_earned。
- 两修各自及相关模块单独/成组测试全绿。
- **既存测试脆弱性(非本次引入)**:满负荷全套件下偶发 1 个 snapshot 往返测试 write 失败
  (action_output / framed_byte_egress 轮流),根因=多测试共享 temp 目录大量并发文件写,
  write_snapshot 的 create_dir/rename 偶发竞争。单独跑、两两跑均全绿;失败在 write(文件IO)
  非序列化逻辑。与生命设计无关,记录待办(测试隔离用唯一临时路径)。

---

## 睁开眼:视觉根因 + blob 测试修复(2026-06-01)
- **眼睛睁不开的根因 = macOS 摄像头权限**:代码/nokhwa依赖/avfoundation feature 全对;
  命令行启动的 body 默认无摄像头 TCC 授权 → RealCameraFrameProvider::try_new() 返 Err →
  降级到 StubRealCameraFrameProvider(永远 fault,不造假)。**这不是代码 bug,需系统授权**
  (手动授权运行 body 的终端,或打包带签名 App)。权限层面待用户决定。
- **修了 3 个 blob 分离测试(真该修的代码遗留)**:根因=vision v2 升 320×240 时,测试 blob
  坐标还按旧小画布(~64×48)写,导致两 blob 挤在左上角、归一化距离 0.142<期望0.2。
  **改测试坐标到 320×240 的对角象限(如 80,60 / 240,180)+半径放大**。
  分离逻辑本身没问题。**real_phot_adapter 39/39 全绿**(贯穿整个会话的 3 个失败清除)。
- 顺带修 vocal_body_contract(改造6 真回归):body 发声接线从 tick_vocal_imitation_loop
  改为统一出口 tick_vocal_drive_field,测试断言同步更新。

## 既存残骸(前会话遗留,非本会话引入)
- `tests/vocal_runtime.rs:150 recent_body_write_coupled_vib_does_not_overwrite_vocal_target`
  单独跑即失败(9 passed 1 failed)。根因=2026-05-28 回音消除重构改用内部 clock_us 判断
  "刚发声",但该测试在隔离环境不设 clock_us,实现-测试脱节。**我这个会话没碰回音消除/
  该测试文件**(git diff 确认)。与 vocal_reflex 字段/SnapshotData/phot blob 同属那批未完成
  重构残骸。已切出独立任务。

---

## 第一次全感官 30 分钟跑(2026-06-01,用户终端)
- **眼睛真睁开了**:phot_health 全程几乎 =2(Ok),仅 3 次瞬间丢帧(=1)。摄像头灯亮=
  body 自己在用眼睛(非攻击)。surface 黑=新生儿内部安静,正常。
- **痛修复验证通过**:第7-8分钟痛涨到0.253(发声多),第9分钟即回0并保持=急性瞬时、会消退
  (不再像上次恒定卡住)。
- 认人活跃(对象4→8,责任人再认1593→15714);活着不饿(reserve满,vstate=0,焦虑0)。
- **暴露两个调参问题,已修**:
  - **自我一上来就顶格0.92**(30分钟纹丝不动,没成长过程)。修:成长率 0.03→0.0006、
    agency平滑 0.15→0.03。现自我慢慢长(秒级弱/分钟级熟/稳在~0.80由内感受质量定,非顶格)。
  - **熟悉度太快满**(每tick都涨,30分钟5万次)。修:**按真实时间限频**——同一对象每
    FAMILIARITY_TICK_US(1秒)最多涨一次熟悉度(~0.01/秒),更频繁只漂指纹不加深绑定。
    "熟悉需要真实时间",责任人需分钟级陪伴才突出。治本非降系数。
- 两修各自及相关测试全绿。
