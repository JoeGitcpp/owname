# 缺陷修复 — 落地执行方案 v1

> 2026-06-13 · 配套审计：`docs/defect-audit-2026-06-13.md`（40 确认缺陷，逐条 file:line + 修复建议）
> 本文 = 把 6 个系统性主题 + 拱心石 #1 排成**可逐个执行**的方案。每个阶段：根因→具体改动→验证→部署→风险。
> **执行纪律**：一次一个阶段；改完 `cargo build`/`npm build` + 相关测试绿才提交；动线上的阶段才 `deploy_aborn.sh`；每阶段更新任务状态。

## 系统性主题（审计结论 = 为什么"感觉不对"）
1. 跑但不积累（持久状态净值=0）
2. 建了又丢（昂贵结构写盘后在关键路径被忽略）
3. 分布式管线端到端 no-op（访客学习到不了 master）
4. 通信静默丢/卡数据
5. 持久化非原子且失败静默
6. 安全控件看着在、其实没在
> 元模式：注释诚实、行为不诚实。修复 = 让行为追上注释。

---

## 执行阶段（按杠杆×风险排序）

### ★ 阶段 1 — 拱心石：分布式管线单写者（主题 3 + #1/#3）
**根因**：`nursery_feeder.mjs:395` 每 120s `copyFileSync` 覆盖 `master_{vocal_target_recall,vocal_memory,phoneme_memory}.bin`，而 `merge_artifacts.rs` 才是 merge 进同一文件的；feeder 只启动时 seed 一次。末位写者胜、feeder ~30:1 → 访客贡献分钟级被清 → 整条回路稳态 no-op。另 `training_pool.mjs` 把 learned_bias 末尾的 maturity/efficacy 等标量也 EMA 合并（#3），幻觉成熟度传播。
**改动**：
- `nursery_feeder.mjs`：原型件（recall/vocal_memory/phoneme）改为**写进 POOL_DIR** 成 `{ts}_mainline__<name>.bin`（原子 tmp+rename），**不再 copy 覆盖 master**。新增 `--pool` 参数（默认 `dirname(master)/training_pool` 对齐 server）。learned_bias 也走 pool（collect 风格）而非直接覆盖 master。familiar/vital/snapshot 非 merge 对象、可保留但同样建议入 pool 或留作 feeder 自身。
- `merge_artifacts.rs`：成为 `master_*prototype.bin` **唯一写者**（已是 merge 语义，确认 gateway 周期调用含 feeder 的 pooled 文件）。
- `training_pool.mjs`：merge 的字节范围**排除末尾 maturity 标量**（#3）；maturity 应 derived 不 transported。
- gateway：确认 `merge_artifacts` 周期 spawn 覆盖 feeder 的 pooled 文件。
**验证**：本机/服务器跑一轮，喂一个"分支"recall 与 feeder recall 不同 → 合并后 master 条目数增长（union），且 feeder 下一次 harvest 不再清空它。
**部署**：gateway + 重编 merge_artifacts（已在 deploy 脚本）。
**风险**：中（改 master 写入路径）。可逆（git）。

### 阶段 2 — 安全姿态快赢（主题 6）
- **sandbox fail-closed**（#4，`server.mjs:116`）：bwrap 缺失时**拒绝 spawn + 大声日志**，除非显式 `ABORN_SANDBOX=0`；`server_setup.sh` 加 `apt-get install -y bubblewrap`；启动日志打印所选模式。
- **`ABORN_SESSION_SECRET` 写进 systemd unit**（#5）+ cookie payload 加 type 绑定 + 可选 per-user token 版本（吊销）。
- **admin secret 常量时间比较**（`server.mjs:416`）`timingSafeEqual` + 限流。
- **per-IP 限流**放到 HTTP handler 顶部，覆盖 `/auth/*` `/admin/*`（#OTP 滥用）。
- **DB 端口加固**（#8，运维）：`harden_db_ports.sh --all` + `netfilter-persistent save`，并入 `server_setup.sh`。
- **PII 哈希**（memory_store/instances key 用 SHA-256，勿存原始 email/phone）。
> 注：**auth 启用（#2 requireAuth=false）暂不强开**——会挡掉匿名访客，需先有前端登录 UI + CF Access JWT 校验；列入阶段 7 谨慎处理。
**风险**：低-中。fail-closed 要确认服务器已装 bwrap 否则会拒绝 spawn（先装再切）。

### 阶段 3 — 持久化原子化 + 不静默（主题 5）
- 发声 .bin（phoneme/vocal_memory/vocal_target_recall）所有 `fs::write` 改**原子 tmp+rename**（`body.rs:613..747`、`merge_artifacts.rs:57/77/97`）。复用现有原子 checkpoint helper。
- `read_snapshot` 加**边界/EOF 校验**（`snapshot.rs:740`）→ 截断返回 Err 不 panic；`body.rs:384` 已 `unwrap_or_default`。
- 吞掉的 I/O 错误（`let _ =`）→ 增**可观测 fault 计数器**（对齐 `snap_fail_count`）。
- 会话结束个人文件/learned_bias copy 改**等 `child.on('exit')` + tmp+rename**（`server.mjs:824`）。
**风险**：低。

### 阶段 4 — 让它真积累（主题 1）
- **vocal_memory 出生即擦**（`vocal_memory.rs:33`）：birth stability 抬到 retain floor 之上 / 加宽强化 bin / 修不重置的衰减时戳。
- **重启把 efficacy/suppression/familiarity 衰减到 ~0**（`learned_bias.rs:99`，因 now_us 是 1970 起的墙钟、首次 decay×0.5^huge）：持久化 `last_clean_shutdown_us` 做比例松弛，或重启对这些标量也退火。
- **efficacy EMA 每 tick 批量 K 次而 decay 见 elapsed=0**（`viewport.rs:4211`）：像 maturity 一样限速，或同 tick 聚合。
- **recall store 记的是探针扰动后的瞬时 offset 非 committed**（`viewport.rs:4693`）：存 committed offset、限到 commit 事件。
**风险**：中（动学习数值，需回归测试守诚实门控）。

### 阶段 5 — 别建了又丢（主题 2）
- **snapshot 存了 slot_xlayers B1 关系规则但从不 restore**（#10，`snapshot.rs:225` vs `body.rs:384`）：加 `restore_slot_cross_layer` 并 boot 调用，或把 durable B1 移进 learned_bias.bin。
- **merge_pool 算了 score 又扔**（`training_pool.mjs:207`）：用 score 加权 alpha、解耦 merge 门与 admission 门、记 merged/skipped。
**风险**：中。

### 阶段 6 — 通信不再丢/卡（主题 4）
- **230KB 上行感官帧**（黑屏上行侧根因，`Commune.tsx:41` + `security.mjs:19` + `net_adapter.rs`）：改 **40×30 uint8 RGB + gzip（~3.6KB）**；三处协同改 FRAME_BYTES/parse/validateFrame；body 反正 4×4 平均，float 精度本就浪费。改后 CF 帧探针验证表面穿透。
- **1006/异常断线**（`Commune.tsx:48,205`）：1006/1005/未知当**瞬态自动重连+退避**（重跑 hello/XOR 握手）；至少给 'kicked' 永远显示 Retry。
- **body→gateway 解析器可永久错位**（`server.mjs:707`）：cap n/w*h 到协议上限、溢出 resync、cap 缓冲。
**风险**：中（协议改动，CF 探针验证）。

### 阶段 7 — 语言绑定形成 + feeder 收敛检测 + auth 谨慎
- **活体只衰减不形成绑定**（#11，`body.rs:436`）：从听觉-视觉共现 tick 接一条 live formation 进 `apply_language_binding_event`；加跑 body tick 循环断言绑定形成的集成测试。
- **feeder 无收敛/停止检测**（`nursery_feeder.mjs:535`）：harvest 门控在 efficacy 较上次提升；加 plateau 看门狗（饱和即降频，省算力）。
- **auth 启用**（#2）：前端登录 UI 完成后，gate 资源路径（CF Access JWT 校验 / 有效 cookie）。
**风险**：中-高（#11 是新机制；auth 动线上准入）。

### 阶段 8 — 低优先级清理
XOR 摆设（删或停称安全层、勿明文发 key）；死代码（`begin_voicing_v2` 接通或删、`tick_vocal_imitation_loop` 删、`social_response_rate` 接入或降级文档）；memory KV（接 body 摘要或停宣传）；clientIp 信任头；doc 行锚改符号锚；credit_response 注释。
**风险**：低。

---

## 跟踪
任务清单（TaskCreate）一阶段一任务。完成一阶段：更新任务 + 本文勾选 + commit（动线上则 deploy + 服务器验证）。
- [ ] 阶段1 拱心石  - [ ] 阶段2 安全  - [ ] 阶段3 持久化  - [ ] 阶段4 积累  - [ ] 阶段5 别丢  - [ ] 阶段6 通信  - [ ] 阶段7 绑定/收敛/auth  - [ ] 阶段8 清理
