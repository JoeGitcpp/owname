# Bionic_v0 服务器工作流重规划 Spec

- 日期: 2026-06-06
- 范围: 服务器(server-joe / `aborn-gateway`)上的(1)持续训练[代码+语音]、(2)摄像头训练成果的提取/清理/释放、(3)注册实例的持久化与定期回收
- 决策基线: **spec-first(先规划再动手)** + **回归单一路径(删除未接线的并行架构)**
- 关联文档: [2026-04-21 global architecture map](2026-04-21-bionic-v0-global-architecture-map.md)

---

## 0. 现状诊断(为什么要重规划)

### 0.1 贯穿全局的核心断层
commit `6acd8fa(prime_manager + harvest_pipeline + auth/instance frontend)` 的模块**建好+单测全绿,但从未接进运行系统**。运行版 `gateway/server.mjs`(最后改动于 `fd45b65`)对 `prime_manager / harvest_pipeline / nursery_feeder` **零引用**,服务器实测同样零引用。服务器跑的仍是更旧的 `training_pool.mjs` 路径。三个问题都坐落在这条"薛定谔架构"断层上。

### 0.2 服务器实测证据(2026-06-06)
| 指标 | 实测 | 结论 |
|---|---|---|
| `aborn-gateway.service` | active/enabled,`node server.mjs` @ `/home/joe/aborn/gateway`,:3900,Restart=always | 网关在跑 |
| `training_pool/` | **0 文件** | 成果从未 harvest 进池 |
| `master_learned_bias.bin` | 18624B,停在 6/3 14:22 | 60min 合并从没搬过新东西 |
| `user-memories/` | 仅 `anon` 1 个 | 无真实注册用户,全 anon |
| gateway 日志 | 全是 anon 的 40s~5min 短连接 | 没有会话够到 5min 质量门 |
| systemd timers | 仅 OS 例行(sysstat/apt/logrotate…) | 无任何训练/回收定时器 |
| 数据库 | 代码零 DB 客户端;Supabase 连着但未授权未使用 | 持久化全靠内存 Map + `.bin` 文件 |

### 0.3 三块各自的真相
- **持续训练**: 服务器上**没有常驻训练体**;body 仅按 WebSocket 每连接临时 spawn(headless),连完即弃。`PrimeManager`(7×24 常驻)建了没接。
  - **代码训练**: 浅——字节回声反射环(`输出=hash(输入)^压力`,`bionic_v0/src/live_byte_closure.rs:124-156`)+ 玩具 16 指令 VM(`bionic_v0/src/bin/sandbox_runner.rs`),仅在本地 `start_training.sh` 跑,**服务器路径无执行器、无代码数据集/课程/目标**。
  - **语音学习**: 真闭环(发声器官+模仿 gap+原型记忆,`bionic_v0/src/viewport.rs` vocal 区)。但 mentor(`scripts/nursery_mentor.py`)依赖 **macOS `say`**,**Linux 服务器跑不了 mentor** → 服务器侧语音只能靠访客麦克风,无课程引导;目标仅到韵律(无音素);依赖只写在注释里的 BlackHole 回环。
  - **governor**: `bionic_v0/src/capability_curriculum_governor.rs` **不在运行回路**,仅 `curriculum_report` 离线调用。分层开门=纸面。
  - mentor 用 macOS `say` + 可选云端 DeepSeek/Anthropic,**不依赖 Qwen:8100**(precraft-designer 是无关产品)。
- **摄像头成果链**: 端到端真跑通(浏览器抽特征→WS XOR 帧→gateway spawn headless body→Rust 轴学习→`learned_bias.bin` 1Hz 原子落盘→断开后质量门→进池→60min 合并),但因会话太短+严格门(≥5min/≥20识别/vocal_efficacy)+`harvest_pipeline` 没接线 → **成果链一次都没走通**。
- **实例生命周期**: 注册表=内存 Map(`gateway/instance_registry.mjs`),**重启全丢**;`requireAuth()` 恒 false→全员 anon;**无实例周期回收器**;注册实例 id 与运行 body session id 是**两套不相干命名空间**(`POST /instances` 不 spawn 任何进程);实例名 UI bug(POST 空 body,名字从不持久化)。

### 0.4 安全问题(并行待处理)
- `aborn-gateway.service` unit 内**明文** `ABORN_SMTP_PASS` 与 `ABORN_SESSION_SECRET` → 应移入 `EnvironmentFile`(chmod 600)。
- `user-memories/anon/` 单目录被所有匿名访客共享读写 → **跨访客记忆串味**。
- WS XOR 仅混淆(自述),机密性完全依赖 WSS/Cloudflare。

---

## 1. 已采纳决策
- **D1 — 删除并行死架构**: 删 `gateway/prime_manager.mjs`、`gateway/harvest_pipeline.mjs` 及二者 `.test.mjs`,回归 `training_pool.mjs` 单一路径。删除前的可复用逻辑已抢救入 §2(原码见 git `6acd8fa`)。
- **D2 — spec-first**: 不再新增"建好不接线"的模块;后续每条改动必须接进 `server.mjs` 或 systemd 才算完成。
- **待定 — `gateway/nursery_feeder.mjs`**: 独立的手动 CLI(`--forever` 喂课程→master),非并行架构的一部分,也无人调用。暂保留,§4 Phase 4 决定是"重塑进单一路径"还是删除。

---

## 2. 抢救清单(从被删模块保留的逻辑,后续 re-home 进单一路径/systemd,而非独立模块)

### 2.1 放松质量门(原 `harvest_pipeline.mjs:13-65`)
解决"成果链空转":
- `RELAXED_MIN_SECS = 60`(原 300);60–300s 短会话**只要有真实 bias delta 即过门**:扫 `learned_bias.bin` 从 byte offset 28 起,每 8 字节 float64-LE,存在 `0.005 < |v| < 1e6` 即 `pass(score=0.3, reason='delta_only_relaxed_gate')`。
- ≥300s 会话仍走原严格 `evaluate_session`。
- 不再强制 `≥20 recognitions`,以 bias delta 为主信号。
> Re-home: 作为 `training_pool.mjs` 的可配置门(env `ABORN_HARVEST_RELAXED=1`),而非新类。

### 2.2 自动合并定时器(原 `harvest_pipeline.mjs:25-31,105-107`)
- `setInterval(mergeNow, mergeIntervalMs)`,默认 120s(对比现网 `ABORN_MERGE_MIN=60min`)。
> Re-home: 现网已有 60min 合并 setInterval(`server.mjs:173-182`);仅需把间隔参数化并下调即可,无需新模块。

### 2.3 常驻 Prime body + 崩溃重启(原 `prime_manager.mjs`)
若决定服务器侧 7×24 自训(§4 Phase 4 开放问题),保留以下设计:
- Prime body 跑在**专用 runtimeDir(非 /tmp,跨重启存活)**,其 `learned_bias.bin` **即全局 master 种子**;新访客会话从它 seed。
- spawn: `body <runtimeDir> --headless <net.sock>`(与访客 body 同二进制,差别在持久+专用目录)。
- 崩溃重启 backoff: `delay = min(5000*crashCount, 30000)`;`maxRestarts=5`,`crashWindow=300s`(静默期后重置 crashCount);超限触发 `onCrash` 放弃。
- 状态读取: `vital_core.bin`(reserve@offset16 f64,vitalState@byte40),`learned_bias.bin`(vocalEfficacy@末尾8字节 f64)。
> Re-home: 作为一个 systemd unit `aborn-prime.service`(`Restart=on-failure`,复用 systemd 的 backoff `RestartSec`/`StartLimit*`),而非 Node 进程内自管子进程。

---

## 3. 目标架构

### 3.1 摄像头成果链(优先级最高,改动最小,先让它真产出)
- 放松质量门(§2.1),让 60–300s 有效会话能进池。
- body 增加 **SIGTERM flush handler**:收到 SIGTERM 时落一次全量 checkpoint 再退出,消除现状"被 kill 掉最后 ~1s 学习、或首个 checkpoint 前死亡=零贡献"(现状仅靠 1Hz 周期 checkpoint,`bionic_v0/src/bin/body.rs:423-447`;退出块 `:489-505` 在 gateway SIGTERM 路径下不执行)。
- 网关启动时**清扫残留** `/tmp/aborn-sessions/*`(防进程崩溃后泄漏)。
- 合并间隔参数化并下调(§2.2)。
- 修 `Permissions-Policy: camera=()` → `camera=(self)`(`server.mjs:345`)。

### 3.2 实例生命周期 + 持久化(用 Supabase 落地)
- 用已连接的 **Supabase** 落:`instances`(id, identity_key, name, state, created_at, last_active_at)、`sessions`、`harvest_index`(成果文件→identity/score/ts)。替换内存 Map,解决"重启全丢"。
- **统一命名空间**: 让 WS 连接绑定到一个已注册 instance(或显式 anon),`ResourceGovernor`(`INSTANCE_MAX`)与 `SessionGuard`(`GLOBAL_MAX`)合一计数。
- 修实例名持久化 bug(`site/src/views/AuthFlow.tsx` POST 空 body → 带 name;`instance_registry` 存 name)。
- **周期回收器**: idle TTL → snapshot-then-evict(先确保成果已 harvest 再回收注册项),记录于 `harvest_index`。
- auth 决策见 §5。

### 3.3 持续训练(代码 + 语音)— 研究性,最后做
- **语音(服务器侧)**: 因 Linux 无 `say`,mentor 课程改用云 TTS 或预生成 PCM 语料喂入 headless body 的 net.sock;或仅依赖访客麦克风(承认无课程)。是否上**音素**级目标(替换现有韵律级 4-band 归约)是关键研究决策。
- **代码训练**: 决定是否真上(代码数据集 + 学习目标 + 表示),还是明确承认现状是"稳态字节回声"并降级命名。
- **governor**: 决定 `capability_curriculum_governor` 是真正接进 body 运行回路(成为课程门),还是降级为离线评估指标(则文档/记忆里"拱心石"的定位需更新)。
- **常驻 prime body**: 是否 7×24 自训(§2.3),独立于访客。

---

## 4. 迁移路径(分阶段,每阶段都必须接线生效)
- **Phase 0(本 spec 同批)**: 删并行死架构(D1),建本 spec。✅
- **Phase 1 — 成果链跑通**: §3.1。验收 = 一次 60–300s 真实摄像头会话 → `training_pool/` 非空 → `master_learned_bias.bin` mtime 前进。
- **Phase 2 — 持久层**: §3.2 的 Supabase schema + 注册/会话/成果索引迁移;requireAuth 决策落地。验收 = 网关重启后注册实例与登录不丢。
- **Phase 3 — 生命周期统一 + 回收器**: §3.2 剩余。验收 = idle 实例被 snapshot-then-evict,成果不丢。
- **Phase 4 — 训练真实性(研究)**: §3.3,单独立项;含 nursery_feeder 去留、音素、代码训练、governor、常驻 prime body。
- **并行 — 安全**: §5。

## 5. 待你拍板的开放问题
1. **代码训练**: 真上数据集+目标,还是先搁置并如实降级命名?
2. **语音(服务器侧)**: 云 TTS mentor / 预生成语料 / 仅访客麦克风?要不要上音素?
3. **governor**: 进运行回路(真课程门)还是降级为离线指标?
4. **auth**: 打开 `requireAuth` 自管,还是正式依赖 Cloudflare Access(并删除半成品 OTP)?
5. **常驻 prime body**: 服务器是否 7×24 自训(独立于访客)?
6. **nursery_feeder.mjs**: 重塑进单一路径,还是删?

---

## 附:关键文件索引
- 运行编排: `gateway/server.mjs` / `gateway/training_pool.mjs`
- 实例/会话: `gateway/instance_registry.mjs` `instance_lifecycle.mjs` `resource_governor.mjs` `ephemeral_session.mjs`
- auth: `gateway/auth.mjs` `phone_auth.mjs` `identity.mjs`
- body 运行时: `bionic_v0/src/bin/body.rs` `net_adapter.rs` `learned_bias.rs` `live_byte_closure.rs`
- 训练脚本: `scripts/start_training.sh` `start_life.sh` `nursery_mentor.py` `install_launchd.sh`
- 前端: `site/src/views/{Commune,AuthFlow,InstanceView}.tsx` `site/src/lib/{media,perception}.ts`
- 部署: `scripts/deploy_aborn.sh` `server_setup.sh`;systemd `aborn-gateway.service`
- **已删(见 git `6acd8fa`)**: `gateway/prime_manager.mjs` `harvest_pipeline.mjs` (+ `.test.mjs`)
