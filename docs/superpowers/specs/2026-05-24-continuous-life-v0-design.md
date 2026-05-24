# Continuous Life v0 — 24/7 持续生命运行设计

> **Status:** FROZEN CANDIDATE
> **Date:** 2026-05-24
> **Parent spec:** [Live Life v0](docs/superpowers/specs/2026-05-24-live-life-v0-design.md) Layer 3
> **Scope:** 从离散 session 转变为 24/7 持续运行，自动恢复，跨天快照归档。

## Boundary

`持续运行`、`生命周期`、`自启动`、`归档` 是工程管理描述，不是 body 本体 ontology。
Body 不知道自己在被"归档"或"管理"——这些是 observer-side 的基础设施。
不改 `AGENTS.md`、`consciousness-stream-architecture.md`、`tier0-base-elements.md`。

禁止：cron 定时触发器写入 body 状态、body-side 运行时间日志、重启计数器作为 body 本体字段。

---

## 三层持续化

### 1. Snapshot 自动归档

当前 snapshot.bin 只有最新一帧。长期运行需要保留历史切面。

**方案：**
- 每小时自动拷贝 `snapshot.bin` → `snapshots/YYYY-MM-DD/HH.bin`
- 每天自动 rollup：保留 00:00 帧，其他小时帧压缩或丢弃
- 由 observer 进程负责（不是 body），body 只写 `snapshot.bin`

### 2. Crash 自动恢复

body 进程异常退出时自动重启，保留上一份 learned_bias checkpoint。

**方案：**
- 简单 wrapper 循环：body 退出 → sleep 2s → 重新启动
- 最多连续重启 5 次（防止死循环崩溃）
- 每次重启加载上次的 learned_bias checkpoint

### 3. macOS 自动启动

**方案：**
- `~/Library/LaunchAgents/com.owname.body.plist` — launchd 配置
- 开机自动启动 body + observer
- KeepAlive=true 保证挂了自动拉起

---

## 文件变更

| 文件 | 变更 |
|------|------|
| `scripts/start_life.sh` | 新增持续运行 wrapper（带 crash recovery） |
| `scripts/install_launchd.sh` | 新增 launchd 安装脚本 |
| `scripts/snapshot_archiver.sh` | 新增定时快照归档脚本 |
| `bionic_v0/src/bin/body.rs` | 可选 duration 后优雅退出（已有）——不改 |

所有变更在 scripts/ 和 docs/，不动 bionic_v0/src/。

---

## 里程碑

| M# | 名称 | 成功标准 |
|----|------|---------|
| C1 | **Snapshot 归档** | 每小时 snapshot 拷贝到归档目录 |
| C2 | **Crash 自恢复** | body 退出后自动重启，learned_bias 恢复 |
| C3 | **24h 稳定** | body 连续 24h 不崩，snapshot 序列完整 |
| C4 | **开机自启** | 重启 Mac 后 body 自动运行 |

---

## 原则守护

| # | Gate | 检查 |
|----|------|------|
| G1 | body 无语义污染 | 新增代码全在 scripts/、docs/，0 行 Rust |
| G2 | 观察分离 | 归档/监控脚本只读 snapshot，不写 body |
| G3 | 数值 substrate 完整 | snapshot 归档保留原始数值 |
