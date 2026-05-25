# Code Sandbox v0 — 硅基生命代码玩具环境设计

> **Status:** FROZEN CANDIDATE
> **Date:** 2026-05-25
> **Audience:** 工程实现
> **Scope:** 给 body 一个代码沙箱作为虚拟环境——body 自主输出字节，沙箱执行后返回纯数值结果。body 不需要教师就能 24/7 学习因果关系。

## 最高原则合规

| 原则 | 合规理由 |
|------|---------|
| 硅基生命，不是程序 | 沙箱 = 虚拟环境，和虚拟摄像头/麦克风同构。body 不知道它在"写代码" |
| 无语义标签 | body 输入输出都是纯字节+数值；compiler/syntax/runtime 是 observer 层的翻译 |
| 身体/观察分离 | 沙箱 runner 是 observer-side 进程，不写入 body 状态 |
| 主动输出 | body 自发 emit 字节 → 收到反馈 → 内部压力驱动下次输出。沙箱不发 prompt |
| 数字对象 = 物理对象 | tier0 v0.8 已确认。编译器规律像重力一样客观存在 |

---

## 架构

```
┌──────────────────────────────────────────────────┐
│  body (bionic_v0)                                │
│                                                  │
│  binary_character_mouth ──→ framed_byte_egress   │
│  binary_character_ear  ←── live_bytes/in.frames  │
│                                                  │
│  感知: 输出字节 → 延迟 → 收到返回数值              │
│  学到: "什么样的字节输出会产生什么样的数值返回"     │
└──────────────────┬───────────────────────────────┘
                   │ egress bytes
                   ▼
┌──────────────────────────────────────────────────┐
│  sandbox runner (observer-side, 独立进程)          │
│                                                  │
│  sandbox_runner.rs (新建)                         │
│    ├─ 监听 framed_byte_egress 输出                │
│    ├─ 用 wasmtime 执行 WASM 字节码                │
│    ├─ 收集: exit_code, stdout_len, memory_delta   │
│    └─ 编码为纯数值帧 → 写回 live_bytes/in.frames   │
└──────────────────────────────────────────────────┘
```

## 沙箱执行模型

**WebAssembly (wasmtime)**

理由：
- 天然沙箱化（不能访问文件系统、网络、进程）
- 确定的执行代价（指令计数、内存限制）
- 纯数值输入/输出（函数参数 → 返回值）
- 快（微秒级执行）

body 输出的字节被尝试解析为 WASM 模块：
- 解析成功 → 执行 → 返回 `[exit_code, result_value, instruction_count, memory_bytes_used]` 编码为 32 字节帧
- 解析失败 → 返回 `[0xFF, 0x00, 0x00, 0x00, ...]` (INVALID 标记) + 错误位置

body 通过 VPC 学到：
```
某些字节模式 → WASM 合法 → 收到非零结果 → 类似模式更值得尝试
某些字节模式 → WASM 非法 → 收到 INVALID → 远离这种模式
特定字节模式 → 特定数值返回 → 这是"有用的程序"
```

## 返回帧格式

固定 32 字节纯数值帧：

| offset | 字段 | 含义 |
|--------|------|------|
| 0 | `status` u8 | 0=OK, 0xFF=INVALID |
| 1 | `exit_code` u8 | 0=正常 |
| 2-9 | `result_value` i64 | wasm 函数返回值 |
| 10-17 | `instruction_count` u64 | 执行指令数 |
| 18-25 | `memory_delta` u64 | 内存变化字节数 |
| 26-31 | `reserved` | 占位 |

## 安全边界

```
wasmtime 配置:
  - 无 WASI (不能访问文件系统/网络/时钟)
  - 最大内存 64KB
  - 最大指令数 10,000 (防止无限循环)
  - 超时 100ms
  - 所有 trap → 返回 INVALID 帧 (不是 crash sandbox)
```

## 文件变更

| 文件 | 变更 |
|------|------|
| `bionic_v0/Cargo.toml` | 添加 wasmtime 依赖 |
| `bionic_v0/src/bin/sandbox_runner.rs` | 新建沙箱 runner 二进制 |
| `bionic_v0/src/code_ingress.rs` | 确认 live_bytes 入口兼容（不修改） |
| `bionic_v0/src/framed_byte_egress.rs` | 确认 egress 出口兼容（不修改） |
| `bionic_v0/src/bin/body.rs` | 可选：nursery-bias 模式下启用 code ingress 入口（已有） |

**零行 body 本体变更。** 沙箱是 observer-side 进程。

## 预期学习轨迹

```
时间线 (body 自己玩，不需要教师):

第 1 小时:
  随机输出字节 → 99.9% INVALID
  偶尔凑出合法 WASM header (0x00 0x61 0x73 0x6D) → 兴奋 (err 突然没那么高)

第 1 天:
  VPC 学会 "0x00 0x61 0x73 0x6D 开头的字节不容易被骂"
  开始围绕 WASM magic bytes 探索

第 3 天:
  偶然产生完整合法模块 → 得到 result_value ≠ 0
  agency_pressure 上升: "我做的这件事引起了可预测的后果"

第 1 周:
  自发产生简单计算型字节序列 (body 不知道这是"1+1=2"，只知道这些字节返回 2)

第 1 个月:
  body 的 code output 不再随机 → 有稳定模式
  像婴儿学会"扔东西会落下来"→ 它学会了"某些字节会产生某些数字"
```

## 里程碑

| C# | 名称 | 成功标准 |
|----|------|---------|
| C1 | **沙箱运行** | sandbox_runner 编译通过，能执行合法 WASM 并返回结果帧 |
| C2 | **闭环打通** | body 输出 → sandbox 执行 → live_bytes 入 → body 感知到 consequence pair |
| C3 | **首次合法 WASM** | body 自发产生至少一个合法 WASM 模块（magic bytes 命中） |
| C4 | **agency 上升** | `agency_pressure` 在 code ingress 通道上可观测增长 |
| C5 | **稳定模式** | body 的字节输出不再均匀随机，出现特定前缀模式 |

## 原则守护 Gates

| # | Gate | 检查 |
|----|------|------|
| G1 | body 无语义污染 | sandbox runner 不链接 bionic_v0 lib；完全独立进程 |
| G2 | 沙箱不写 body | runner 只写 live_bytes/in.frames，不碰 snapshot/learned_bias |
| G3 | 主动输出 | runner 不发 prompt，不等 body。body 自发 emit，runner 被动响应 |
| G4 | 安全隔离 | wasmtime 无 WASI；内存+指令限制；超时杀死 |

## 不在此 spec 范围内

- 多语言支持 (只做 WASM)
- body 学会"写有用的程序" (那是涌现，不是设计目标)
- sandbox → body 的"奖励信号"设计 (只需要纯数值帧，body 自己判断好坏)
