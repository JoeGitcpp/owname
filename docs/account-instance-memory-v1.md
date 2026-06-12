# 账号登录 + 长期持久实例 + 按 AI 持久记忆 v1

> 状态：后端实现完成并端到端验证 2026-06-12 · 作者：Joe + Fable
> 原则：全部**增量**，**登录保持可选**（`requireAuth` 仍 false）→ 对现有匿名访客**零影响**，上线低风险。

## 三个功能

### 1. 账号登录（跨重启稳定）
- OTP 流程（邮箱 `auth.mjs` + 手机 `phone_auth.mjs`）原已存在。
- **无状态签名 cookie**（改造）：身份+过期编码进 `base64url(payload).hmac(payload)`，验证从签名 payload 重建——不再依赖内存 sessions Map，**登录跨网关重启存活**。
- **共享持久 secret**（`session_secret.mjs`）：env `ABORN_SESSION_SECRET` → 持久文件 `.session_secret` → 生成并落盘（带告警）。修掉「重启换随机 secret 把所有人登出」+「邮箱/手机用不同 secret」。
- `/auth/check` 改用 `resolveIdentity`，邮箱**或**手机 cookie 任一有效即 authed（供 SPA 恢复登录态）。
- 新增 `POST /auth/logout`：清除两个 cookie。

### 2. 长期持久实例
- **`InstanceRegistry` 落盘**（`instance_registry.mjs`）：可选 `persistPath`，构造时加载、每次变更原子写（tmp+rename）。**返回用户的实例跨重启存活**（原内存版重启全丢——核心 bug）。新增 `name`/`setName`/`touch`。
- `server.mjs` 用 `ABORN_INSTANCES`（默认 `../instances.json`）构造；`POST /instances` 收 `{name}`（≤64 字符）存入。
- 「记得你」语义：回访 authed 用户的 body 仍按其用户目录 seed（familiar_objects 等）+ 主线声学技能 → AI 带着对你的记忆和共享技能起步。
- 注：**未做活进程 warm-resume**（SIGSTOP/CONT 让 body 跨连接常驻）——文件 seed 模型已交付「记得+保留学习」，warm-resume 是后续优化。

### 3. 按 AI 持久记忆
- `MemoryStore`（原孤立）**接线**：专用目录 `ABORN_MEMORY_DIR`（默认 `../user-memory-kv`），按 identityKey 命名空间的 JSON KV。
- HTTP 端点（认证）：`GET /memory`（harvest/导出）、`GET /memory/keys`、`PUT /memory/:key {value}`。
- 会话结束写 `_meta`：访问次数、最近见面、实例名（结构化「跨会话记得你」层，区别于二进制学习文件）。
- 注：**body↔记忆的 socket 桥未做**（body 进程内读写 KV 记忆）。body 自身学习经按用户二进制文件持久化——那是基质级记忆，已工作。KV 层目前供网关/前端读写结构化事实。

## 验证
- 全网关测试 **150 绿**（含新增 InstanceRegistry 持久化 4 项）。
- 端到端 smoke：手机 OTP→cookie→PUT memory→POST instance(带名)→**重启新进程**→登录/实例/记忆**全部存活**；logout 后转 unauthorized；`/auth/check` 认手机登录。

## 安全（仍是公开服务，注意）
- ✅ OTP 限流、cookie HttpOnly+Secure+SameSite=Strict、PII 仅存 SHA-256、body bwrap 沙箱（原有）。
- ✅ 无状态 cookie HMAC 签名 + 常量时间比较；secret 持久化。
- ⚠️ 部署务必设 `ABORN_SESSION_SECRET`（强随机）；`ABORN_ADMIN_SECRET` 强随机（admin 端点无限流，靠 secret 强度）。
- ⚠️ 无状态 cookie 改格式 → 上线后现有 cookie 失效，用户需重新登录一次（一次性）。

## 前端 SPA（已接并上线 2026-06-12）
- `App.tsx`：启动 `GET /auth/check` → authed 则 `GET /instances/me` 自动恢复进持久实例（跨刷新、回访重连到自己的 AI）。
- `AuthFlow.tsx`：建实例发 `{name}`（原发 `{}` 丢名字）。
- `InstanceView.tsx`：退出按钮 `POST /auth/logout` 真清 cookie（原只回首页）。
- 已部署，线上 bundle 确认含 `/auth/check`。注意：landing 的「进入」=匿名 commune、「登录」=持久实例——要持久必须走登录。

## 待办（后续优化）
- body↔记忆 socket 桥（body 进程内读写 KV；当前 body 学习经按用户二进制文件持久化=基质级记忆已工作）。
- 活进程 warm-resume、admin 端点限流、记忆查看页 UI。

## 部署清单（线上 aborn.live）
1. `git pull`（main）。
2. 设 env：`ABORN_SESSION_SECRET`（强随机，**务必**）、确认 `ABORN_ADMIN_SECRET` 强。可选 `ABORN_INSTANCES`/`ABORN_MEMORY_DIR` 自定路径（默认即可）。
3. `deploy_aborn.sh` 同步网关 + 重启（参 server-deploy-ops）。
4. 前端如改：`site` 构建后部署 dist。
5. 验证：`/auth/check`、登录后 `/instances/me`、重启后仍在。
