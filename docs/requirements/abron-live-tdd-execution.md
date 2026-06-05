# TDD 执行文档：abron.live PRD v1.0 全量实现

**对应 PRD**：abron-live-prd-v1.md
**日期**：2026-06-05
**测试运行器**：
- Gateway (Node.js ESM): `node --test gateway/**/*.test.mjs`
- bionic_v0 (Rust): `cargo test --release --lib` / `cargo test --release --bin body`
- Frontend (React): `npm test --prefix site` (vitest，需 M2 时补配置)

**约定**：
- 每个模块先写测试（RED），再写实现（GREEN），通过后提交
- 与时间相关的测试注入假时钟（`clock: () => fakeNow`），不 sleep
- 与 body binary 相关的集成测试用 `timeout 5 /path/to/body /tmp/test-dir` + Python socket 验证
- 验收标准 (AC-Fx.y) 直接对应 PRD § 验收标准

---

## 阶段总览

| 里程碑 | 模块数 | 测试数(估) | 对应 PRD |
|--------|--------|-----------|---------|
| M1 主体活起来 | 7 | ~85 | F1.1/1.2/1.3/1.4/1.5, F2.1/2.3, I2 |
| **M1.5 感知+发声升级** | **3** | **~50** | **F7(L1), F9 生物发声** |
| M2 让人参与 | 5 | ~60 | F3, F4.1–4.4, F8 听觉 |
| M3 认主人+规模化 | 4 | ~45 | F5, I1, I3 |
| M4 能力加速 | 3 | ~30 | F6.2, 空间, 治理器 |
| **合计** | **19** | **~220** | 全部 |

---

## M1 — 主体活起来 + 不缺空间

### 前置操作：I2.1 LVM 扩容（ops 手术，无单元测试）

**执行人**：Joe（需 root）

```bash
# 1. 确认未分配空间
sudo vgdisplay ubuntu-vg | grep "Free  PE"
# 2. 扩 LV（吃满剩余空间）
sudo lvextend -l +100%FREE /dev/ubuntu-vg/ubuntu-lv
# 3. 在线扩文件系统
sudo resize2fs /dev/ubuntu-vg/ubuntu-lv
# 4. 验收
df -h / | tail -1   # 应显示 ~900G
```

**集成验收**：`df -h /` 使用率 < 50%。完成后继续 M1 其余模块。

---

### M1-1. `reserve_balance` — reserve 生存平衡（F1.4）

**对应文件**：`bionic_v0/src/vital_drive_field.rs` 改动 + 新增 Rust 测试

**问题**：`survival_gated_urge` 在 reserve 0.7 时 gain=0.82，仍允许大量发声 → exertion 累积 → reserve 崩。  
**修复方向**：提早介入——`self_preservation_pressure` 在 reserve < 0.5 时开始显著抑制，而非等到 reserve→0。

**API 变更**（不改接口，只改 `self_preservation_pressure` 曲线）：
```rust
// 当前: unit((1 - reserve)*0.6 + anxiety*0.4)
// 目标: 非线性，reserve 0.5 以下快速上升
pub fn self_preservation_pressure(&self) -> f64 {
    if self.state == VitalState::Standby { return 1.0; }
    // 改为平方律：低储量比高储量贡献更大压力
    let reserve_pressure = (1.0 - self.reserve).powi(2);
    unit(reserve_pressure * 0.7 + self.vital_anxiety * 0.3)
}
```

**测试文件**：`bionic_v0/src/vital_core.rs` 内的 `#[cfg(test)]` 模块  
**测试清单**（新增 8 个）：

| # | 场景 | 期望 |
|---|------|------|
| 1 | reserve=1.0 时 self_preservation_pressure | <0.05（几乎 0） |
| 2 | reserve=0.7 时 | <0.20（轻微抑制）|
| 3 | reserve=0.5 时 | ~0.25（开始介入）|
| 4 | reserve=0.2 时 | >0.60（强抑制）|
| 5 | reserve=0.0 时 | ~1.0 |
| 6 | Standby 时 | ==1.0 |
| 7 | survival_gated_urge(0.6, pressure@reserve0.7) | >0.48（仍允许发声） |
| 8 | survival_gated_urge(0.6, pressure@reserve0.2) | <0.25（强压）|

**运行**：
```bash
cd bionic_v0 && cargo test --release --lib reserve_balance 2>&1 | tail -5
```

**验收**（AC-F1.4）：nursery 喂养 300s，reserve 最低谷 >0.10，主体不长期 Standby。

---

### M1-2. `prime_manager.mjs` — 主体进程管理器（F1.1）

**文件**：
- `gateway/prime_manager.mjs`（新建）
- `gateway/prime_manager.test.mjs`（新建）

**职责**：
- 启动/监控主体 body 进程（独立 runtime，不在 `/tmp/aborn-sessions`）
- 崩溃自动重启（最多 3 次/5 分钟）
- 读取主体 `learned_bias.bin` 作为全局 master seed
- 暴露 `getPrimeStatus()` 供 `/admin/prime/status` 调用

**API 表面**：
```js
class PrimeManager {
  constructor(opts)                // { bodyBin, runtimeDir, masterBiasPath, clock }
  async start()                   // 启动进程，返回 Promise<void>
  stop()                          // SIGTERM，清理
  isAlive()                       // boolean
  restartCount()                  // number（本轮）
  getStatus()                     // { reserve, nPrototypes, vocalEfficacy, mergeCount, uptime }
  onCrash(cb)                     // 注册崩溃回调
  getMasterBiasPath()             // string — prime runtime dir 的 learned_bias.bin
}
```

**测试清单**（15 个）：

| # | 场景 | 类型 |
|---|------|------|
| 1 | start() 启动后 isAlive()=true | 正常路径 |
| 2 | stop() 后 isAlive()=false | 正常路径 |
| 3 | 进程退出后自动重启（mock process exit） | 恢复 |
| 4 | 3 次崩溃后 restartCount()=3 | 状态 |
| 5 | 5 分钟内崩溃超过 3 次 → onCrash 触发，不再重启 | 保护 |
| 6 | 时间窗口过期后重启计数重置（假时钟） | 时序 |
| 7 | getMasterBiasPath() 返回 prime runtime 内的路径 | 路径 |
| 8 | getStatus() 读取 vital_core.bin → reserve 字段 | 状态 |
| 9 | getStatus() 读取 learned_bias.bin → vocal_efficacy | 状态 |
| 10 | getStatus() 文件不存在时返回 null 字段而不抛 | 边界 |
| 11 | getStatus().uptime 随时间增加（假时钟） | 时序 |
| 12 | 构造时 runtimeDir 不存在 → 自动创建 | 边界 |
| 13 | prime 种子文件从 masterBiasPath 复制进 runtimeDir | 种子 |
| 14 | 多次 start() 幂等（不重复启动） | 幂等 |
| 15 | stop() 幂等（不抛异常） | 幂等 |

**运行**：
```bash
cd gateway && node --test prime_manager.test.mjs 2>&1 | tail -8
```

**验收**（AC-F1.1）：`master_learned_bias.bin` mtime 持续滚动；主体 7 天存活。

---

### M1-3. `nursery_adaptive.mjs` — 自适应喂养增强（F1.2）

**文件**：
- `gateway/nursery_feeder.mjs`（更新现有文件）
- `gateway/nursery_feeder.test.mjs`（新建）

**新增能力**：
- 读 prime runtime 的 `vital_core.bin` 获取 reserve，动态调整喂养强度
- 读能力探针（capability_curriculum_governor 输出）推进课程阶段
- `AdaptiveFeeder` 类封装策略，可注入 clock + 假 reserve

**API 新增**：
```js
class AdaptiveFeeder {
  constructor({ clock, reserveReader, probeReader })
  getEnergy(reserve)              // f64 ∈ [0.1, 0.6] — 自适应强度
  getRestMs(reserve)             // 课间休息毫秒，reserve 低时更长
  shouldAdvanceCurriculum(probe) // boolean
  currentPhase()                 // 'tones'|'pairs'|'rhythm'|'words'
}
```

**测试清单**（12 个）：

| # | 场景 | 期望 |
|---|------|------|
| 1 | reserve=0.9 → getEnergy() 接近最大(0.55) | 高能 |
| 2 | reserve=0.3 → getEnergy() 接近最小(0.12) | 低能 |
| 3 | reserve=0.0 → getEnergy() = 0（静默） | 保护 |
| 4 | reserve=0.9 → getRestMs() 较短(3000) | 勤快 |
| 5 | reserve=0.2 → getRestMs() 较长(20000) | 休息 |
| 6 | probe 未达阈值 → shouldAdvanceCurriculum()=false | 卡门 |
| 7 | probe 超阈值 → shouldAdvanceCurriculum()=true | 进阶 |
| 8 | 初始阶段 = 'tones' | 初始 |
| 9 | 连续 3 次 advance → 阶段到 'rhythm' | 进阶链 |
| 10 | reserveReader 抛异常时 getEnergy() 返回默认中值 | 容错 |
| 11 | 饥荒期（reserve 持续低）喂养总量 < 非饥荒期 50% | 节流 |
| 12 | 饱腹期喂养不超最大强度（能量不溢出 [0,1]） | 上限 |

**运行**：
```bash
cd gateway && node --test nursery_feeder.test.mjs 2>&1 | tail -8
```

---

### M1-4. `harvest_pipeline.mjs` — 训练回收管线（F2.1 + F2.3）

**文件**：
- `gateway/harvest_pipeline.mjs`（新建）
- `gateway/harvest_pipeline.test.mjs`（新建）
- `gateway/training_pool.mjs`（修改：放宽质量门）

**质量门新规则**：
```
原规则：session_secs ≥ 300 AND topRecog ≥ 20
新规则：
  (bias_delta > DELTA_THRESHOLD AND n_events ≥ MIN_EVENTS)
  OR (session_secs ≥ 60 AND bias_delta > DELTA_THRESHOLD * 3)
  -- 丢弃：全静音、全 Standby、无任何 bias 变化
MIN_EVENTS = 5   (原 20)
DELTA_THRESHOLD 保持 0.005
```

**API**：
```js
class HarvestPipeline {
  constructor({ poolDir, primeMgr, mergeIntervalMs, clock })
  evaluate(runtimeDir, startedAt, endedAt)  // → { pass, score, reason }
  collect(runtimeDir, sessionId)            // → poolFilePath | null
  mergeNow()                               // 立即触发一次 EMA 合并
  stats()                                  // { poolSize, totalMerges, lastMergeAt }
}
```

**测试清单**（18 个）：

| # | 场景 | 期望 |
|---|------|------|
| 1 | 60s 会话 + bias delta + n_events=6 → pass | 新质量门 |
| 2 | 30s 高质量（delta*3）→ pass | 增量回收 |
| 3 | 300s 但全 Standby → fail | 旧 bug 修复 |
| 4 | 全静音（无 VIB 事件）→ fail | 噪声过滤 |
| 5 | 120s + bias delta=0 → fail（没有真实学习）| 无增量 |
| 6 | pass 后 collect() 在 poolDir 创建文件 | 存储 |
| 7 | collect() 文件名含 timestamp_sessionId | 命名 |
| 8 | mergeNow() 把池文件 EMA 合并进 prime learned_bias | 合并 |
| 9 | mergeNow() 空池时无错误返回 { merged:0 } | 边界 |
| 10 | mergeNow() 后池文件被清理（已合并的旧文件）| 清理 |
| 11 | stats().totalMerges 每次 mergeNow 后+1 | 计数 |
| 12 | stats().lastMergeAt 更新（假时钟）| 时间 |
| 13 | mergeIntervalMs 到期后自动触发合并（假时钟）| 定时 |
| 14 | 合并后 prime 的 vocal_efficacy ≥ 合并前（EMA 不降好值）| 单调 |
| 15 | 匿名会话只回收通用 bias（不含个人字段）| 隐私 |
| 16 | 个人命名空间字段不出现在池文件中 | 隐私 |
| 17 | 质量加权：高分会话贡献权重 > 低分（3× score）| 加权 |
| 18 | 并发 collect() 不创建重复文件（文件名唯一）| 并发 |

**Rust 测试（training_pool.rs）**：同步修改 `bionic_v0/src/bin/body.rs` 里的 `evaluate_session` 调用，并在 Rust 侧补 3 个新测试覆盖新阈值。

**运行**：
```bash
cd gateway && node --test harvest_pipeline.test.mjs 2>&1 | tail -8
cd bionic_v0 && cargo test --release --lib evaluate_session 2>&1 | tail -5
```

**验收**（AC-F2.1/2.3）：有访客时训练池持续进出；prime bias mtime 在合并后更新。

---

### M1-5. `prime_status_api` — 主体状态路由（F1.5）

**文件**：`gateway/server.mjs`（新增路由 `/admin/prime/status`）

**路由**：
```
GET /admin/prime/status
  Header: X-Admin-Secret: <secret>
  Response: {
    alive: bool,
    reserve: f64,
    vitalState: 'Alive'|'Standby',
    nPrototypes: int,
    vocalEfficacy: f64,
    restartCount: int,
    uptimeSec: int,
    mergeCount: int,
    poolSize: int,
    masterBiasUpdatedAt: iso8601
  }

GET /admin/prime/status (no auth) → 403
```

**测试清单**（8 个，加入 server 集成测试文件 `gateway/admin.test.mjs`）：

| # | 场景 |
|---|------|
| 1 | 无 secret header → 403 |
| 2 | 错误 secret → 403 |
| 3 | 正确 secret → 200 含 alive 字段 |
| 4 | prime alive → alive=true |
| 5 | prime 未启动 → alive=false，其他字段 null |
| 6 | reserve/nPrototypes 字段类型正确（number/int） |
| 7 | masterBiasUpdatedAt 是 ISO 格式或 null |
| 8 | 并发 10 次请求不崩溃（stress micro-test） |

**运行**：
```bash
cd gateway && node --test admin.test.mjs 2>&1 | tail -6
```

---

### M1-6. `storage_layout.sh` — 存储分层（I2.2/I2.3）

**文件**：
- `scripts/storage_layout.sh`（新建，idempotent）
- `scripts/storage_cleanup.sh`（新建，systemd timer 调用）

**storage_layout.sh 执行内容**：
```bash
# 挂载 /data (nvme0n1) — 若未挂
# 挂载 /mnt/cold (sda1 重格式化 ext4) — 若未挂
# 建目录结构:
#   /data/aborn/pool          ← 训练池（温层）
#   /data/aborn/user-memories ← 用户记忆（温层）
#   /mnt/cold/aborn/archives  ← 冷层归档
#   /mnt/cold/aborn/logs      ← 日志轮转
# 更新 /etc/fstab
# 更新 gateway systemd 环境变量
```

**storage_cleanup.sh 定时任务（systemd timer）**：
```
每天 03:00:
  - 清理超 7 天的 /tmp/aborn-sessions/*
  - 轮转 gateway.log → /mnt/cold/aborn/logs/
  - 归档超 30 天的训练池文件到 /mnt/cold/aborn/archives/
  - 检查各分区使用率，>85% 发 warn 日志
```

**验收测试**（bash assertions）：
```bash
# scripts/test_storage.sh
assert_mounted /data
assert_mounted /mnt/cold
assert_dir_exists /data/aborn/pool
assert_dir_exists /data/aborn/user-memories
df -h / | awk 'NR==2{gsub(/%/,"",$5); exit ($5>50)?1:0}'  # / < 50%
systemctl is-enabled aborn-cleanup.timer
```

---

### M1 集成测试（`gateway/m1_integration.test.mjs`）

**场景**：启动 prime_manager → 注入 5 分钟假课程喂养 → 检查：

| # | 验收指标 | 判定 |
|---|---------|------|
| 1 | prime 存活（isAlive=true） | ✅/❌ |
| 2 | prime reserve 最低谷 >0.1 | ✅/❌ |
| 3 | harvest_pipeline stats().totalMerges >0 | ✅/❌ |
| 4 | master_learned_bias.bin mtime 比启动前新 | ✅/❌ |
| 5 | /admin/prime/status → nPrototypes>0 至少一次 | ✅/❌ |

```bash
cd gateway && node --test m1_integration.test.mjs --timeout 360000 2>&1 | tail -10
```

---

## M2 — 让人能参与

### M2-1. `sms_adapter.mjs` — 国内短信适配器（F3.1）

**文件**：
- `gateway/sms_adapter.mjs`（新建）
- `gateway/sms_adapter.test.mjs`（新建）

**设计**：transport-agnostic，与 `phone_auth.mjs` 的 `smsSender` 接口对接。

**API**：
```js
// 策略接口（已有）
type SmsSender = (phone: string, otp: string) => Promise<void>

// 工厂
function createAliyunSms({ accessKeyId, accessKeySecret, signName, templateCode })
  → SmsSender

function createTencentSms({ secretId, secretKey, sdkAppId, signName, templateId })
  → SmsSender

function createConsoleSms()    // dev fallback（已有，打印到控制台）
function createMockSms()       // test helper，捕获发送记录
```

**测试清单**（12 个）：

| # | 场景 |
|---|------|
| 1 | createMockSms() 捕获 phone + otp |
| 2 | mock 调用 2 次 → sentMessages 长度 2 |
| 3 | mock clear() 清空记录 |
| 4 | createAliyunSms 缺配置 → 构造时抛 InvalidConfig |
| 5 | createAliyunSms 正常构造 → 返回函数 |
| 6 | aliyun sender 调用时 options 含 SignName + TemplateCode |
| 7 | aliyun sender 调用时 PhoneNumbers 格式正确 |
| 8 | aliyun send 失败（mock 网络）→ 抛 SmsDeliveryError |
| 9 | createTencentSms 缺配置 → 构造时抛 |
| 10 | tencent sender options 含 TemplateId + SignName |
| 11 | phone_auth.mjs ABORN_SMS_SEND=aliyun → 使用 aliyun adapter |
| 12 | phone_auth.mjs ABORN_SMS_SEND=tencent → 使用 tencent adapter |

**运行**：
```bash
cd gateway && node --test sms_adapter.test.mjs 2>&1 | tail -6
```

---

### M2-2. `auth_flow.test.mjs` — 认证流程端到端（F3 API 层）

**测试完整注册→实例创建流**（不走真实短信，使用 mock sender）：

**测试清单**（14 个）：

| # | 场景 |
|---|------|
| 1 | POST /auth/request-phone {phone} → 200 ok |
| 2 | POST /auth/verify-phone {phone,otp} 正确 → 200 ok + Set-Cookie |
| 3 | cookie 有效 → POST /instances {name} → 201 含 instanceId |
| 4 | 实例创建后 GET /instances/me → 200 含 name 字段 |
| 5 | 同手机号再次 POST /instances → 409 instance_exists |
| 6 | DELETE /instances/me → 200，再 GET → instance:null |
| 7 | DELETE 后可重新创建 |
| 8 | 邮箱路径：request + verify + 实例创建同样走通 |
| 9 | 无 cookie POST /instances → 401 |
| 10 | 错误 OTP → verify 返回 ok:false（不设 cookie）|
| 11 | 3 次错误 OTP → 锁定，第 4 次返回 null |
| 12 | 限流：4 次 request-phone 同号 → 第 4 次 429 |
| 13 | 注册流 ≤3 次 HTTP 调用（request→verify→instances）| 简洁 |
| 14 | 完整 round-trip 响应时间 <500ms（无真实网络）| 性能 |

**运行**：
```bash
cd gateway && node --test auth_flow.test.mjs 2>&1 | tail -8
```

---

### M2-3. 前端组件（F4）— React + Vitest

**新文件**（site 目录）：
```
site/src/views/Landing.tsx          ← 改版（加访客入口 + 登录引导卡）
site/src/views/AuthFlow.tsx         ← 新建（OTP 表单）
site/src/views/InstanceView.tsx     ← 新建（专属实例界面）
site/src/views/AuthFlow.test.tsx    ← 新建
site/src/views/Landing.test.tsx     ← 新建
site/src/views/InstanceView.test.tsx← 新建
site/vitest.config.ts               ← 新建（jsdom 环境）
```

**Landing.test.tsx 测试清单**（10 个）：

| # | 场景 |
|---|------|
| 1 | 渲染主按钮"直接进入，开始陪它" |
| 2 | 渲染登录引导卡（含"认你为主人"文案）|
| 3 | 点击主按钮 → onEnterGuest() 回调触发 |
| 4 | 点击登录卡 → 展示 AuthFlow |
| 5 | 中文/英文切换 i18n |
| 6 | 无障碍：主按钮有 aria-label |
| 7 | 不渲染任何登录表单（访客直入） |
| 8 | 渲染时不发起任何 HTTP 请求 |
| 9 | 小屏（375px）主按钮仍可见 |
| 10 | 大屏（1440px）布局不破坏 |

**AuthFlow.test.tsx 测试清单**（12 个）：

| # | 场景 |
|---|------|
| 1 | 初始渲染：手机号/邮箱输入框可见 |
| 2 | 切换 tab：手机 ↔ 邮箱 |
| 3 | 输入手机号 → 点"获取验证码" → API mock 被调用 |
| 4 | 验证码输入框在请求后出现 |
| 5 | 输入错误验证码 → 显示错误提示 |
| 6 | 验证成功 → 进入起名阶段（step 3） |
| 7 | 输入名字 → 点完成 → onComplete(name) 回调 |
| 8 | 整个流程仅 3 步 |
| 9 | loading 状态：按钮 disabled |
| 10 | 网络错误 → 错误提示不崩溃 |
| 11 | 30 秒倒计时（重发验证码按钮） |
| 12 | 倒计时到 0 → 重发按钮可用 |

**InstanceView.test.tsx 测试清单**（8 个）：

| # | 场景 |
|---|------|
| 1 | 顶部显示实例名 |
| 2 | commune canvas 存在（表面可见）|
| 3 | "认主人"状态指示器存在 |
| 4 | 未识别时显示中性提示 |
| 5 | 识别高置信时提示变化 |
| 6 | "注销/切换"按钮存在 |
| 7 | 访客实例（无名字）不崩溃 |
| 8 | 断线后 UI 有重连提示 |

**运行**：
```bash
cd site && npm run test 2>&1 | tail -10
# 配置: site/vitest.config.ts + @testing-library/react
```

---

### M2 集成验收

```bash
# 端到端：从手机→OTP→实例→Commune 全流程（E2E，可选 playwright）
node scripts/e2e_auth_flow.mjs --phone-mock --headless 2>&1 | tail -5
```

- 从落地页到"实例已创建" ≤30s ≤3 次输入（人工测量）
- 访客 0 操作直接开始（自动化：检查 Landing 不显示登录表单）

---

## M3 — 认主人 + 规模化

### M3-1. `owner_binding.mjs` — 主人绑定（F5）

**文件**：
- `gateway/owner_binding.mjs`（新建）
- `gateway/owner_binding.test.mjs`（新建）

**设计**：主人绑定是**持久化到用户命名空间**的会话元数据，不进主体，不含原始音视频。

**API**：
```js
class OwnerBinding {
  constructor({ memoryStore })   // 注入 MemoryStore 实例
  recordSession(identityKey, sessionStats)
    // sessionStats: { durationSec, nVibEvents, topFamiliarity, meanVocalEfficacy }
    // 更新 ownerProfile: visitCount, totalDuration, famThreshold
  getOwnerProfile(identityKey)   // → { visitCount, famThreshold, isEstablished }
  isOwnerEstablished(identityKey)// boolean (visitCount ≥ 3 AND famThreshold > 0.3)
  clearOwner(identityKey)        // 删除绑定数据（用户注销）
}
```

**测试清单**（12 个）：

| # | 场景 |
|---|------|
| 1 | 首次 recordSession → visitCount=1 |
| 2 | 3 次 recordSession → visitCount=3 |
| 3 | isOwnerEstablished 在 visitCount<3 → false |
| 4 | isOwnerEstablished 在 visitCount≥3 + fam>0.3 → true |
| 5 | famThreshold 随高 familiarity 会话上升 |
| 6 | famThreshold 随低 familiarity 会话缓降（不剧跌）|
| 7 | getOwnerProfile 未知身份 → null |
| 8 | clearOwner 后 getOwnerProfile → null |
| 9 | clearOwner 不影响其他用户数据 |
| 10 | 数据持久化：新建 OwnerBinding 同 memoryStore → 读回相同数据 |
| 11 | recordSession 并发不破坏计数（串行化写） |
| 12 | sessionStats 个人字段（面孔/声音原始值）不被存储 |

**运行**：
```bash
cd gateway && node --test owner_binding.test.mjs 2>&1 | tail -6
```

**WS handler 修改**：会话结束时调用 `ownerBinding.recordSession()`，并把 profile 传给 prime admin 暴露的主人状态接口。

---

### M3-2. GPU 基准测试（I1）— Rust 性能测试

**目标**：确定 wgpu splat vs CPU splat 的吞吐倍数，设定容量模型。

**文件**：
- `bionic_v0/benches/gaussian_splat.rs`（新建，cargo bench）
- `bionic_v0/src/phot_gaussian_gpu.rs`（新建，wgpu 后端）

**测试清单**（Rust benchmark + 单元测试，9 个）：

| # | 场景 | 类型 |
|---|------|------|
| 1 | CPU splat 64 Gaussians 48×36 → 输出长度正确 | 单元 |
| 2 | GPU splat 64 Gaussians 48×36 → 输出与 CPU 数值一致（误差<1e-5）| 数值对照 |
| 3 | GPU init 失败（无 GPU） → fallback 到 CPU | 降级 |
| 4 | CPU splat 吞吐 benchmark（cargo bench，基线）| 性能 |
| 5 | GPU splat 吞吐 benchmark（目标 >10× CPU）| 性能 |
| 6 | GPU 批处理 N=8 → 吞吐线性（效率 >70%）| 扩展 |
| 7 | VRAM 使用 <500MB（12GB RTX 3060 的 4%，留余量）| 资源 |
| 8 | AR 预测器（全轴）GPU 版数值与 CPU 一致 | 数值对照 |
| 9 | substrate 逐轴 GPU 版吞吐 benchmark | 性能 |

**容量模型验收（运行 benchmark 后导出）**：
```
1 GPU unit (3060 12GB) ≈ N_concurrent body  
其中 N = VRAM_headroom / per_body_vram + CPU_thread_ratio
→ 写入 /admin/prime/capacity 路由供 resourceGovernor 热更
```

**运行**：
```bash
cd bionic_v0 && cargo bench gaussian_splat 2>&1 | grep "ns/iter\|Mb/s"
cd bionic_v0 && cargo test --release gaussian_gpu 2>&1 | tail -6
```

---

### M3-3. `capacity_governor.mjs` — GPU 容量驱动（I3）

**文件**：
- `gateway/capacity_governor.mjs`（新建）
- `gateway/capacity_governor.test.mjs`（新建）

**API**：
```js
class CapacityGovernor {
  constructor({ resourceGovernor, primeManager, benchmarkPath, clock })
  loadBenchmark(path)            // 从 cargo bench 导出的 JSON 读容量公式
  computeLimit()                 // → int（当前应设的 body 上限）
  applyToGovernor()              // 调用 resourceGovernor.setLimit(n)
  startAutoAdjust(intervalMs)    // 定期调整
  stopAutoAdjust()
}
```

**测试清单**（8 个）：

| # | 场景 |
|---|------|
| 1 | loadBenchmark 读取合法 JSON → computeLimit() 返回 >0 |
| 2 | benchmark 缺失 → 返回保守默认值 8 |
| 3 | GPU 可用 → computeLimit() > CPU 纯算结果 |
| 4 | applyToGovernor() 调用 resourceGovernor.setLimit |
| 5 | applyToGovernor() prime 进程始终不受限（独立预算） |
| 6 | startAutoAdjust 定时触发（假时钟）|
| 7 | stopAutoAdjust 停止定时器 |
| 8 | 限制下调时 resourceGovernor.suspended() 正确 |

---

## M4 — 能力加速

### M4-1. `capability_governor_wire` — 治理器接线（F6.2）

**文件**：
- `bionic_v0/src/bin/body.rs`（新增调用）
- `bionic_v0/src/capability_curriculum_governor.rs`（现有，补调用）
- Rust 测试：`#[cfg(test)]` 在 `capability_curriculum_governor.rs`

**问题**：治理器 830 行，9 个测试通过，但 body.rs **零调用** → 门控是假的。

**修复**：在 body.rs 的主循环 `run_global_refresh_cycle` 中调用 `vp.tick_capability_governor(t)`，让它真正门控各层的 push_observation。

**测试清单**（新增 Rust 集成测试，10 个）：

| # | 场景 |
|---|------|
| 1 | 初始层 0 开门 → push_observation 到达 tier-0 轴 |
| 2 | 层 1 未开门 → push_observation 到高层轴被拦截 |
| 3 | 编码器探针超阈值 → 层 1 解锁 |
| 4 | 层 1 解锁后 → 深度轴 push_observation 可到达 |
| 5 | 关系探针超阈值 → 层 2 解锁 |
| 6 | 强制降级（探针跌落）→ 层自动重锁 |
| 7 | body 运行 100 ticks 后 capability_governor 至少被调 100 次 |
| 8 | 治理器不影响 VIB/PHOT 轴（tier-0 基础感知不被门控）|
| 9 | 层 0 始终开门（物理先验不被关）|
| 10 | 探针值从 snapshot 可读（observer 可观测）|

**运行**：
```bash
cd bionic_v0 && cargo test --release --lib capability_curriculum 2>&1 | tail -6
cd bionic_v0 && cargo test --release --bin body capability_governor_wired 2>&1 | tail -5
```

---

### M4-2. `virtual_neck_wire` — 空间感知接线（F6.3）

**文件**：`bionic_v0/src/bin/body.rs`（接线），`bionic_v0/src/virtual_neck.rs`（已有）

**现状**：`virtual_neck.rs` + `depth_observer.rs` 有实现但 body.rs 零引用。

**修复**：在 `run_global_refresh_cycle` 中调用 `vp.tick_virtual_neck(t)`，让头部运动学模型真正驱动深度估计。

**测试清单**（新增 8 个）：

| # | 场景 |
|---|------|
| 1 | tick_virtual_neck 被 body 主循环调用（无 panic） |
| 2 | 静止（无平移）→ 深度标注"不可观测" |
| 3 | 有平移输入 → 深度估计有非零输出 |
| 4 | efference copy 来自 ActionOutputEvent（不凭空产生）|
| 5 | 深度 ρ 存在 snapshot → observer 可读 |
| 6 | 深度不可观测时，不污染其他轴 |
| 7 | 虚拟脖子 tick 不超过 5ms/次（不阻塞主循环）|
| 8 | 多次 tick 无内存泄漏（arena 计数不增长）|

---

### M4-3. `probe_dashboard` — 能力探针可观测（F6 验收）

**文件**：
- `gateway/probe_dashboard.mjs`（新建）
- `gateway/probe_dashboard.test.mjs`（新建）

**API**：
```js
class ProbeDashboard {
  constructor({ primeMgr, clock })
  readProbes()    // 从 prime snapshot 读所有探针值
  // → { vocal: f64, hearing: f64, vision: f64, depth: f64,
  //      relation: f64, attention: f64, curiosity: f64 }
  history(days)   // 读历史趋势（从日志），返回 [{ts, probes}]
  isMonotone(probe, days)  // 该探针近 N 天单调上行？→ bool
  // 注意：仅报告，不断言——能力是涌现的，不被强制
}

// 路由: GET /admin/prime/probes → probeDashboard.readProbes()
```

**测试清单**（7 个）：

| # | 场景 |
|---|------|
| 1 | readProbes() 返回 7 个探针字段 |
| 2 | 字段值在 [0,1] 内 |
| 3 | prime 未运行 → readProbes() 返回全 null（不抛）|
| 4 | history(7) 返回 ≤7 个数据点 |
| 5 | isMonotone：一周全上行 → true |
| 6 | isMonotone：有下行 → false（不误报）|
| 7 | /admin/prime/probes 路由 → 200 含探针对象 |

---

## 全量运行命令

```bash
# ── M1 全部 ────────────────────────────────────────────────
cd gateway && node --test \
  prime_manager.test.mjs \
  nursery_feeder.test.mjs \
  harvest_pipeline.test.mjs \
  admin.test.mjs \
  m1_integration.test.mjs 2>&1 | tail -12

cd bionic_v0 && cargo test --release --lib 2>&1 | tail -6

# ── M2 全部 ────────────────────────────────────────────────
cd gateway && node --test sms_adapter.test.mjs auth_flow.test.mjs 2>&1 | tail -8
cd site && npm run test 2>&1 | tail -10

# ── M3 全部 ────────────────────────────────────────────────
cd gateway && node --test owner_binding.test.mjs capacity_governor.test.mjs 2>&1 | tail -6
cd bionic_v0 && cargo bench gaussian_splat 2>&1 | grep "ns/iter"
cd bionic_v0 && cargo test --release gaussian_gpu 2>&1 | tail -5

# ── M4 全部 ────────────────────────────────────────────────
cd bionic_v0 && cargo test --release --lib capability_curriculum 2>&1 | tail -5
cd bionic_v0 && cargo test --release --lib virtual_neck 2>&1 | tail -5
cd gateway && node --test probe_dashboard.test.mjs 2>&1 | tail -5

# ── 全量一键（CI）─────────────────────────────────────────
bash scripts/run_all_tests.sh
```

---

## 验收矩阵（PRD § 验收标准 → 测试映射）

| PRD 验收标准 | 对应测试 | 里程碑 |
|-------------|---------|-------|
| AC-F1.1: master_learned_bias mtime 持续滚动 | M1 集成测试 #4 | M1 |
| AC-F1.1: 主体 7 天存活 | prime_manager 测试 #1 + systemd timer | M1 |
| AC-F1.4: reserve 最低谷 >0.10 | reserve_balance 测试 #7/8 + M1集成 #2 | M1 |
| AC-F1.4: n_prototypes 能持续 >0 | M1 集成测试 #5 | M1 |
| AC-F2.1: 训练池有访客时持续进出 | harvest_pipeline 测试 #6/7 + M1集成 #3 | M1 |
| AC-F2.3: 合并日志显示 bias 更新 | harvest_pipeline 测试 #8/11/14 | M1 |
| AC-F2.4: 个人数据不进主体 | harvest_pipeline 测试 #15/16 | M1 |
| AC-F3: ≤30s ≤3次输入完成注册 | auth_flow 测试 #13 + AuthFlow UI 测试 #8 | M2 |
| AC-F4: 访客 0 点击登录即开始 | Landing 测试 #7 | M2 |
| AC-F4: 登录用户重连延续记忆 | InstanceView 测试 #1 + owner_binding 测试 #10 | M2 |
| AC-F5: attachment 高于陌生人 | owner_binding 测试 #3/4/5 | M3 |
| AC-I1: GPU 热路径运行，CPU 下降 | gaussian_gpu benchmark #4/5/6 | M3 |
| AC-I2: `/` 使用率 <50% | storage_layout test assertion | M1 |
| AC-F6: 探针周维度上行 | probe_dashboard isMonotone 测试 #5 | M4 |
| AC-F7: Gaussian 聚类稳定 | visual_resolution 测试 #12 | M1.5 |
| AC-F9: 元音频谱重心差 >200Hz | vocal_synthesis 测试 #14 | M1.5 |
| AC-F8: 男女声 VIB 差异 >0.3 | hearing_upgrade 测试 #9 | M2 |

---

## M1.5 — 感知升级 + 生物发声（M1 完成后立即执行）

> 这一阶段纯 CPU，无 GPU 依赖，不改动任何后端路由——只改感知帧格式和声音合成。  
> **优先级**：生物发声（F9）最快见效（纯 Rust 替换，1-2天），视觉升级（F7）约3-4天，听觉升级（F8）约1.5天。三者 M1.5 全部落地，不分阶段，一次到位。

---

### M1.5-1. `perceptual_upgrade` — 视觉 + 听觉 CPU 最大化（F7 + F8）

**设计决策**：帧格式同时升级视觉和听觉，统一更新三端（browser / gateway / Rust），确保新旧帧不混用。

**改动文件**：
- `site/src/lib/perception.ts`（画布 → 160×120，FFT 2048，64 mel 带 + 8 aux 特征，RGB 三通道）
- `site/src/views/Commune.tsx`（帧格式常量）
- `gateway/security.mjs`（`SENSES_FRAME_BYTES`）
- `bionic_v0/src/net_adapter.rs`（`GRID_W/H/CHANNELS`，`BANDS`，`FRAME_BYTES`，VIS 16 区域 × 8 轴）
- `bionic_v0/src/lib.rs`（`VIB_BANDS=16`，`VIS_REGIONS=16`，`VIS_AXES_PER_REGION=8`，`AXIS_COUNT=161`）
- `bionic_v0/src/phot_gaussian.rs`（`GAUSS_REC_W=320`，`GAUSS_REC_H=240`，`GAUSS_CAP=512`，Rayon 并行，RGB 三通道）

**新常量（唯一目标规格）**：
```
GRID_W = 160,  GRID_H = 120,  GRID_CHANNELS = 3  // 57,600 像素 RGB
BANDS  = 64                                         // mel-scale
AUX    = 8                                          // 时频特征
FRAME_BYTES = 1 + (2 + 64 + 2 + 8)×4 + 160×120×3×4
           = 1 + 304 + 230,400 = 230,705 bytes

GAUSS_REC_W = 320, GAUSS_REC_H = 240  // 76,800 像素（2× 上采样自浏览器）
GAUSS_CHANNELS = 3                     // RGB
GAUSS_PIX = 76,800
GAUSS_CAP = 512
VIB_BANDS = 16
VIS_REGIONS = 16 (4×4)
VIS_AXES_PER_REGION = 8
AXIS_COUNT = 161
```

**测试文件**：
- `gateway/perceptual_upgrade.test.mjs`（JS 端帧格式 + 带宽验证）
- `bionic_v0/src/phot_gaussian.rs` `#[cfg(test)]`
- `bionic_v0/src/lib.rs` 轴常量测试更新

**测试清单**（共 24 个）：

**JS / 帧格式（8 个）**：

| # | 场景 | 期望 |
|---|------|------|
| 1 | `SENSES_FRAME_BYTES` = 230705 | 唯一正确值 |
| 2 | `validateFrame(230705字节, 0x01)` → true | 通过 |
| 3 | `validateFrame(3137字节, 0x01)` → false（旧帧被拒绝）| 版本门 |
| 4 | `validateFrame(230706字节, 0x01)` → false | padding 防御 |
| 5 | `packSenses` 输出恰好 230705 字节（含 3 通道网格）| 帧构造 |
| 6 | 帧内 R/G/B 三平面偏移各自正确（305, 77105, 153905）| 通道布局 |
| 7 | `gateway maxPayload ≥ 230705 + 64` | 不被截断 |
| 8 | 20Hz × 230705 bytes < 4.7 MB/s（带宽预算内）| 网络可行 |

**Rust Gaussian（11 个）**：

| # | 场景 | 期望 |
|---|------|------|
| 9 | `GAUSS_PIX = 320×240 = 76800` | 常量 |
| 10 | `GAUSS_CAP = 512` | 常量 |
| 11 | `GaussianField::new()` 分配 512 × 3 通道 | 内存正确 |
| 12 | `tick_grid_rgb(320×240×3)` 不 panic | 正常路径 |
| 13 | 10 帧渐变 RGB 输入后 `active_count > 0` | 场启动 |
| 14 | 100 帧后 `active_count > 32`（512 cap 的 6% 以上）| 丰富重建 |
| 15 | `splat()` 返回长度 `= 3 × 76800` | 输出维度 |
| 16 | 320×240 RGB 的 MSE < 48×36 灰度（相同输入）| 质量更高 |
| 17 | 红色区域和蓝色区域在 splat 输出颜色可区分（ΔE > 20）| 色彩工作 |
| 18 | Rayon 并行版与串行版输出数值一致（误差 < 1e-6）| 并行正确 |
| 19 | Rayon 版 tick_grid 速度 > 串行版 3×（12 核期望 ~8-10×）| 性能验证 |

**视觉轴 + 听觉轴（5 个）**：

| # | 场景 | 期望 |
|---|------|------|
| 20 | `AXIS_COUNT = 161` | 常量 |
| 21 | 16 区域感知：`push_observation` 调用 16×8 = 128 次/帧 | 轴充满 |
| 22 | `VIB_BANDS = 16`；64 mel 带 → 16 VIB 轴映射无重叠 | 映射正确 |
| 23 | `familiar_objects` 音频指纹维度 = 16 | 指纹升级 |
| 24 | 升级后 lib tests 全绿（无轴范围越界）| 无回归 |

**运行**：
```bash
cd gateway && node --test perceptual_upgrade.test.mjs 2>&1 | tail -6
cd bionic_v0 && cargo test --release --lib perceptual_upgrade 2>&1 | tail -6
cd bionic_v0 && cargo test --release --lib 2>&1 | tail -3  # 全量回归
```

**验收**（AC-F7/F8）：
- `active_count > 32` 在有人脸输入时
- Rayon 并行 > 3× 串行速度
- 3 实例并发 CPU < 80%（集成测试量实际负载）

---

### M1.5-2. `vocal_synthesis` — 生物型发声合成（F9）

**改动文件**：
- `bionic_v0/src/net_adapter.rs`（`sample_vocal_pcm` 函数替换）
- 新建 `bionic_v0/src/vocal_synthesis.rs`（Rosenberg + formant + 有机纹理）

**核心新模块 `vocal_synthesis.rs`**：

```rust
// 声门源：Rosenberg-C 脉冲（非线性，有丰富谐波）
pub fn rosenberg_c(t_in_period: f64, open_quotient: f64) -> f64

// 两极共振峰 IIR 滤波器
pub struct FormantFilter { /* a1, a2, b0 系数 */ }
impl FormantFilter {
    pub fn new(freq_hz: f64, bandwidth_hz: f64, sample_rate: f64) -> Self
    pub fn process(&mut self, x: f64) -> f64
}

// 粉红噪声生成器（1/f 频谱，比白噪声更自然）
pub struct PinkNoise { /* Voss-McCartney 算法，无外部 RNG 依赖 */ }
impl PinkNoise { pub fn next(&mut self) -> f64 }

// 主合成函数（替换原 sample_vocal_pcm）
pub fn synthesize_biological(
    organ: &VocalOrganState,
    reserve: f64,
    phase: &mut f64,        // 相位累积（跨帧连续）
    f1_filter: &mut FormantFilter,
    f2_filter: &mut FormantFilter,
    pink: &mut PinkNoise,
    n_samples: usize,
    sample_rate: u32,
) -> Vec<f32>
```

**测试文件**：`bionic_v0/src/vocal_synthesis.rs` 内 `#[cfg(test)]`

**测试清单**（共 20 个，升级版：5 formants + 44100Hz）：

| # | 场景 | 期望 |
|---|------|------|
| 1 | `rosenberg_c(0.0, 0.6)` → 0.0（相位起点）| 初值正确 |
| 2 | `rosenberg_c` 一周期积分 ≈ 0（无直流偏置）| 无 DC |
| 3 | `rosenberg_c` FFT：前 8 次谐波各自 > -40dB（vs 正弦只有 1 次）| 谐波丰富 |
| 4 | `rosenberg_c(t, oq=0.5)` 闭相占比 50%（开商控制有效）| 参数控制 |
| 5 | `FormantFilter::new(500, 80, 44100)` 在 500Hz 处增益最大 | F1 正确 |
| 6 | 5 个串联 FormantFilter（F1–F5）后频谱有 5 个峰 | 全声道 |
| 7 | resonance_pos=0.0 vs 1.0：F1/F2 频率差 > 900Hz | 元音空间宽 |
| 8 | 相邻元音（Δpos=0.3）频谱重心差 > **400Hz** | 元音可辨 |
| 9 | `PinkNoise` 均值 ≈ 0，频谱斜率 ≈ -3dB/octave | 1/f 特性 |
| 10 | `synthesize_biological` breath=0.0 → 全零输出 | 静默保护 |
| 11 | 不同 pitch_tension → 基频差 > 50Hz | 音高控制 |
| 12 | 高 noise_mix → 噪声能量占比 > 30% | 气息感 |
| 13 | 低 noise_mix → 谐波能量占比 > 95% | 清晰发声 |
| 14 | 颤音：连续 1024 帧基频标准差 > **3Hz** | 有颤音 |
| 15 | reserve=0.1 颤音深度 < reserve=0.9 的 25% | 生存联动 |
| 16 | Shimmer：相邻帧 RMS 差 > 0.5%（非恒幅）| 有机纹理 |
| 17 | 跨帧相位连续（`phase` 保持）→ 帧接缝无爆音（DC offset < 0.001）| 连续性 |
| 18 | 起音：breath 从 0 升到 0.8 时，前 10ms 输出 < 0.4（指数攻击）| 自然起音 |
| 19 | 收音：breath→0 后 20ms 内幅度衰减到 < 0.01 | 自然收音 |
| 20 | 1024 样本帧 at **44100Hz** → 约 23ms（升级后帧时长）| 采样率 |

**运行**：
```bash
cd bionic_v0 && cargo test --release --lib vocal_synthesis 2>&1 | tail -8
```

**integration smoke**（Python 验证发出的声音不再是纯正弦）：
```bash
ssh server-joe "
  python3 -c \"
import socket, struct, numpy as np
# ... 连接 body socket, 收 512 帧 vocal PCM
# 计算 FFT 谐波分布：生物声音 >5 个显著谐波
# 纯正弦只有 2-3 个
\"
"
```

**验收**（AC-F9）：
- 不同 `resonance_position` 下发音频谱重心差 >200Hz（可测）
- 有颤音（相邻帧基频标准差 >1Hz）
- 听感测试：主观评分 >3/5（人工）

---

### M1.5-3. `hearing_upgrade` — 听觉 32 带（F8，归 M2）

> 此模块改动帧格式，需与前端 M2 一起发布，**测试先写，实现在 M2 时一并提交**。

**改动文件**：
- `site/src/lib/perception.ts`（`BANDS = 32`，升 FFT，mel-scale 滤波）
- `site/src/views/Commune.tsx`（帧格式：`BANDS = 32`）
- `gateway/security.mjs`（`SENSES_FRAME_BYTES` 再次更新）
- `bionic_v0/src/net_adapter.rs`（`BANDS = 32`，`VIB_BANDS = 8`）
- `bionic_v0/src/lib.rs`（`VIB_BANDS` 常量）

**新帧格式**：
```
BANDS = 32（+20），辅助特征 4 个
帧字节 = 1 + (32 + 2 + 2 + 4)*4 + 96*72*4
       = 1 + 160 + 27648 = 27,809 bytes（在 L1 基础上增 96 bytes）
```

**测试清单**（`gateway/hearing_upgrade.test.mjs`，12 个）：

| # | 场景 | 期望 |
|---|------|------|
| 1 | `SENSES_FRAME_BYTES` = 27809 | 常量正确 |
| 2 | validateFrame(27809字节) → true | 新帧通过 |
| 3 | packSenses 在 `BANDS=32` 下输出 27809 字节 | 浏览器端 |
| 4 | 辅助特征 spectral_centroid ∈ [0,1] | 范围 |
| 5 | 辅助特征 zero_crossing_rate ≥ 0 | 非负 |
| 6 | 高频输入 → spectral_centroid 接近 1 | 正确方向 |
| 7 | 低频输入 → spectral_centroid 接近 0 | 正确方向 |
| 8 | Rust VIB_BANDS = 8（更新后）| 常量 |
| 9 | 32→8 频带映射：男声（低 F0 ~150Hz）集中在 band 0-2 | 区分 |
| 10 | 女声（高 F0 ~250Hz）集中在 band 3-5 | 区分 |
| 11 | familiar_objects 指纹从 4→8 维（不破坏已有数据格式）| 兼容 |
| 12 | 升级后 parse_frame 不 panic（BANDS=32 适配）| 安全 |

---

## 更新后验收矩阵（M1.5 新增行）

| PRD 验收标准 | 对应测试 | 里程碑 |
|-------------|---------|-------|
| AC-F7 L1: Gaussian 聚类 active_count>10 | visual_resolution 测试 #13/14 | M1.5 |
| AC-F7: 帧格式全三端一致（browser/gateway/Rust）| visual_resolution 测试 #1-8 | M1.5 |
| AC-F9: 元音频谱重心差 >200Hz | vocal_synthesis 测试 #10 | M1.5 |
| AC-F9: 有颤音（相邻帧基频标准差 >1Hz）| vocal_synthesis 测试 #13 | M1.5 |
| AC-F9: 生存联动（低 reserve→颤音减弱）| vocal_synthesis 测试 #14 | M1.5 |
| AC-F8: 男女声 VIB 差异 >0.3 | hearing_upgrade 测试 #9/10 | M2 |

---

## 更新后执行顺序建议

```
M1（前置）: [同前]

M1.5（M1 完成后立即，纯 CPU）:
  Day 6:   vocal_synthesis.rs（Rosenberg + formant + 有机纹理）→ 测试绿
           → sample_vocal_pcm 替换 → 部署 → 人工听感验证
  Day 7-8: visual_resolution（96×72，三端同步更新帧格式）→ 测试绿
           → 部署 → observer 验证 active_count 提升

M2（M1.5 完成后）:
  Day 9:   hearing_upgrade（32 带，与前端 M2 一起）
  Day 9:   sms_adapter + 选定短信商
  Day 10-12: 前端 Landing + AuthFlow + InstanceView → 部署

[M3/M4 同前]
```

| 问题 | 测试待补充 |
|------|----------|
| 国内短信商选定后 | sms_adapter.test.mjs 加真实 mock（不发真短信）|
| 合并频率定下后 | harvest_pipeline 测试 #13 补具体 intervalMs 值 |
| 质量门新阈值 | harvest_pipeline 测试 #1/2 参数化 |
| GPU 容量公式 | 压测后写入 capacity_governor.test.mjs #1 |
| 推广并发峰值 | 负载测试脚本（scripts/load_test.mjs，M3 后补）|

---

## 执行顺序建议

```
M1（前置，立即）:
  Day 1:   I2.1 扩 LVM（ops）→ 验证磁盘释放
  Day 1-2: reserve_balance Rust 修复 → 测试绿
  Day 2-3: prime_manager + nursery_adaptive → 测试绿
  Day 3-4: harvest_pipeline 质量门放宽 → 测试绿
  Day 4-5: prime_status_api → 测试绿
  Day 5:   M1 集成测试全绿 → 部署 prime 常驻服务

M2（串行）:
  Day 6:   sms_adapter + 选定短信商
  Day 7-9: 前端 Landing + AuthFlow + InstanceView → 测试绿
  Day 9-10: M2 集成验收 → 部署

M3（并行可行）:
  owner_binding (2天)   ‖   GPU benchmark (2天)
  → capacity_governor (1天)
  → M3 部署

M4（持续推进，与推广并行）:
  capability_governor_wire → virtual_neck_wire → probe_dashboard
```

---

*关联文档：[abron-live-prd-v1.md](abron-live-prd-v1.md)、[instance-management-tdd-plan.md](instance-management-tdd-plan.md)*
