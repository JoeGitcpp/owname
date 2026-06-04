# TDD 执行计划：用户实例管理系统

**对应需求**：instance-management-v1.md  
**日期**：2026-06-04  
**测试运行器**：`node --test`（Node.js 内置，无额外依赖）

---

## 模块拆分与依赖关系

```
identity.mjs              ← 无依赖（纯逻辑 + 内存状态）
instance_registry.mjs     ← 无依赖
instance_lifecycle.mjs    ← 无依赖（纯函数状态机）
resource_governor.mjs     ← 无依赖
memory_store.mjs          ← node:fs（持久化）
ephemeral_session.mjs     ← 无依赖（内存 + 注入时钟）
```

模块间无循环依赖；server.mjs 最终组装它们。

---

## 各模块 TDD 计划

### 1. `identity.mjs` — 身份标识 + OTP

**职责**：邮箱/手机号归一化、OTP 生命周期（生成→校验→会话 token）、频率限制与锁定。

**API 表面**：
```js
normalizeEmail(email)                    // → string | throws
normalizePhone(phone)                    // → E.164 string | throws
identityKey(type, value)                 // → "email:..." | "phone:..." | throws
class OtpStore(opts)
  .requestOtp(type, value, sender)       // → Promise<void> | throws RateLimit
  .verifyOtp(type, value, otp)           // → token | null
  .checkSession(token)                   // → { type, value } | null
```

**测试清单**（共 11 个）：
| # | 场景 | 类型 |
|---|------|------|
| 1 | normalizeEmail 小写+trim | 正常路径 |
| 2 | normalizeEmail 无 @ → throws | 边界 |
| 3 | normalizePhone 带格式化 → E.164 | 正常路径 |
| 4 | normalizePhone 太短 → throws | 边界 |
| 5 | identityKey email/phone 格式 | 正常路径 |
| 6 | identityKey 未知类型 → throws | 边界 |
| 7 | requestOtp 调用 sender，code 为 6 位数字 | 正常路径 |
| 8 | verifyOtp 正确 OTP → 返回 token | 正常路径 |
| 9 | verifyOtp 错误 OTP → null | 边界 |
| 10 | checkSession 有效 token → identity | 正常路径 |
| 11 | checkSession 无效 token → null | 边界 |
| 12 | verifyOtp 超过 maxAttempts → 锁定 | 安全 |
| 13 | requestOtp 超过 maxRequests → throws | 安全 |
| 14 | verifyOtp TTL 过期（注入假时钟）→ null | 时序 |
| 15 | session TTL 过期（注入假时钟）→ null | 时序 |

**假时钟注入**：`new OtpStore({ clock: () => fakeNow })` 避免真实 sleep。

---

### 2. `instance_registry.mjs` — 实例 CRUD

**职责**：创建/查询/销毁实例，强制同一身份唯一性约束。

**API 表面**：
```js
class InstanceRegistry
  .create(identityKey)      // → { id, identityKey, state, createdAt } | throws Duplicate
  .get(id)                  // → instance | null
  .getByIdentity(key)       // → instance | null
  .destroy(id)              // → void
  .setState(id, state)      // → void | throws
  .list()                   // → instance[]
  .count()                  // → number
```

**测试清单**（共 10 个）：
| # | 场景 |
|---|------|
| 1 | create → 返回带 id 和 pending_verification 状态的对象 |
| 2 | create 重复身份 → throws |
| 3 | create 销毁后同身份可再建 |
| 4 | get 已知 id → 数据 |
| 5 | get 未知 id → null |
| 6 | getByIdentity 已知身份 → 实例 |
| 7 | getByIdentity 未知身份 → null |
| 8 | destroy 移除记录 + 解除身份占用 |
| 9 | count 随创建/销毁变化 |
| 10 | 不同 identityKey 相互独立 |

---

### 3. `instance_lifecycle.mjs` — 状态机

**职责**：纯函数状态机，非法跳转抛异常。

**状态图**：
```
pending_verification ──verify──→ running
pending_verification ──destroy─→ destroyed
running ──suspend──→ suspended
running ──destroy──→ destroyed
suspended ──resume──→ running
suspended ──destroy──→ destroyed
(任何状态 + 非法事件 → throws)
```

**API 表面**：
```js
transition(state, event)  // → nextState | throws
STATES                    // string[]
```

**测试清单**（共 12 个）：每条合法边 1 个测试，每条非法边 1 个 throws 测试。

---

### 4. `resource_governor.mjs` — 资源配额 + 挂起队列

**职责**：全局运行上限、热更新 `setLimit`、FIFO 挂起队列、销毁时自动出队。

**API 表面**：
```js
class ResourceGovernor(limit, opts?)
  .limit                       // getter
  .setLimit(n)                 // → instanceId[] 需被挂起的列表（热更新副作用）
  .canStart()                  // → boolean
  .onStart(instanceId)         // → void | throws
  .onDestroy(instanceId)       // → nextToResume | null
  .onSuspend(instanceId)       // → void
  .suspendQueue()              // → instanceId[] FIFO 顺序
  .running() .suspended() .total()
```

**测试清单**（共 11 个）：
| # | 场景 |
|---|------|
| 1 | canStart 低于上限 → true |
| 2 | canStart 达上限 → false |
| 3 | onStart 达上限 → throws |
| 4 | onDestroy 释放槽位 |
| 5 | onDestroy 有挂起队列 → 返回下一个 id |
| 6 | onDestroy 队列空 → null |
| 7 | onSuspend 移出 running 加入队列 |
| 8 | suspendQueue FIFO 顺序 |
| 9 | setLimit 上调 → 解锁槽位 |
| 10 | setLimit 下调 → 返回超出的实例列表 |
| 11 | running()/suspended()/total() 反映真实状态 |

---

### 5. `memory_store.mjs` — 命名空间持久记忆

**职责**：文件系统存储，每用户独立目录，支持非破坏性 harvest。

**API 表面**：
```js
class MemoryStore(basePath)
  .write(identityKey, key, value)   // → void
  .read(identityKey, key)           // → value | null
  .listKeys(identityKey)            // → string[]
  .deleteNamespace(identityKey)     // → void
  .harvest(identityKey)             // → { [key]: value }（快照，不删数据）
```

**测试清单**（共 11 个）包含：写读往返、命名空间隔离、harvest 非破坏性、跨进程重启持久性（两个 MemoryStore 实例共享同一 basePath）。

---

### 6. `ephemeral_session.mjs` — 临时会话

**职责**：无持久化，基于注入时钟的 idle 超时，per-IP 并发限制。

**API 表面**：
```js
class EphemeralSessionManager(opts?)
  .create(ip)          // → sessionId | throws PerIpLimit
  .heartbeat(id)       // → void
  .destroy(id)         // → void
  .countForIp(ip)      // → number
  .size()              // → number
```

**测试清单**（共 11 个）包含：超时后 sweep（假时钟）、heartbeat 重置计时器、destroy 释放 IP 槽位、expired sessions 在 create 时自动清理。

---

## 执行顺序

```
Phase 0  写测试文件（RED — 全部失败，impl 文件不存在）
Phase 1  写实现文件（GREEN — 逐模块通过）
Phase 2  node --test gateway/*.test.mjs  全绿
```

## 运行命令

```bash
# 单个模块
node --test gateway/identity.test.mjs

# 全部
node --test gateway/identity.test.mjs \
          gateway/instance_registry.test.mjs \
          gateway/instance_lifecycle.test.mjs \
          gateway/resource_governor.test.mjs \
          gateway/memory_store.test.mjs \
          gateway/ephemeral_session.test.mjs
```

---

*下一步：API spec（HTTP 路由 + WebSocket 握手扩展）*
