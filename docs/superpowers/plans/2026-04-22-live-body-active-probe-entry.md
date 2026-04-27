# Live Body Active Probe Entry Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在现有持续运行底座、`action_output` substrate、`action_consequence` 后果窗口与时间桥上，让“最小合法主动探针”以 body 内生方式进入 live loop。 

**Architecture:** 新增一个纯数值/时间的 `probe_law` 子底座，把“是否发起/延续/衰减探针”写成结构关系更新，而不是固定目标清单或单阈值数字门。`body` orchestrator 只调用该子底座刷新与写出映射；后果窗口与配对继续复用现有 `action_consequence` 路径，不新造高层模块。

**Tech Stack:** Rust (`std`, `RwLock`, `AtomicU64`), existing `Viewport` / `action_output` / `action_consequence` / snapshot / observer, Cargo integration tests.

---

## Hard Constraints (Must Hold)

- 仅允许数值/时间/更新约束字段；禁止对象目标清单、结果类别清单、固定探索范围字段。
- 禁止把主动性写成“单阈值 + 单延迟 + 单开关”三元控制器。
- 探针发起与抑制必须来自结构关系（相对强弱、历史分位、持续性），不是固定世界内容。
- body 进程继续禁止人类可读输出；观测仅走 snapshot/observer。

---

## File Structure

**Create**
- `bionic_v0/src/probe_law.rs`
- `bionic_v0/tests/live_probe_entry.rs`

**Modify**
- `bionic_v0/src/lib.rs`
- `bionic_v0/src/viewport.rs`
- `bionic_v0/src/bin/body.rs`
- `bionic_v0/src/snapshot.rs`
- `bionic_v0/src/bin/observer.rs`

**Do Not Modify In This Plan**
- `bionic_v0/src/language_binding.rs`
- `bionic_v0/src/adapter.rs`（后果窗口累计与时间桥已在 live PHOT 路径中）
- 任何命名资格门、离散编号层、对象目标白名单

---

## Chunk 1: Probe Law Substrate (No World Content)

### Task 1: 引入最小 `probe_law` 状态与刷新入口

**Files:**
- Create: `bionic_v0/src/probe_law.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: 在 `probe_law.rs` 定义纯 substrate 状态**  
  只允许：相对未闭合强度、连续性强度、抑制强度、合成驱动力、阶段、时间戳、短历史 ring。禁止目标对象/结果标签/意义字段。

- [ ] **Step 2: 在 `Viewport` 挂载 `probe_law: RwLock<...>`**  
  作为与 `action_output` / `action_consequence` 并列子树；默认值全零/空历史。

- [ ] **Step 3: 新增 `refresh_probe_law(now_us)`**  
  输入仅来自现有底座：`hl.last_avg_mse`、`global_reasoning`、`global_reasoning_memory_summary`、health、当前 write/window 形态。  
  更新规则使用“历史相对位置 + 结构连续性 + 抑制关系”而非固定阈值门。

- [ ] **Step 4: 加入边界断言**  
  在 `probe_law.rs` 注释与测试中明确：该层不是目标选择器、不是结果判定器、不是高层行为模块。

- [ ] **Step 5: 运行编译检查**  
  Run: `cargo build --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml`

### Task 2: 将 `probe_law` 映射到 `action_output` 事件

**Files:**
- Modify: `bionic_v0/src/probe_law.rs`
- Modify: `bionic_v0/src/viewport.rs`

- [ ] **Step 1: 新增 `tick_probe_output_from_law(now_us)`**  
  仅做 `Idle/Active/Decaying` 到 `Begin/Adjust/Decay/Stop` 的映射与幂等保护。

- [ ] **Step 2: 保持“小步可试 + 可撤回”**  
  写出幅度由当前驱动力的相对位置连续映射得到；不得引入固定目标范围。

- [ ] **Step 3: 保持“无固定时机数字门”**  
  不增加“超过 N 毫秒必发起/必终止”类固定规则；仅允许基于历史相对结构的转移。

- [ ] **Step 4: 运行聚焦单测（先 RED 后 GREEN）**  
  Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml live_probe_entry -- --nocapture`

---

## Chunk 2: Enter Live Body Orchestrator

### Task 3: 将主动探针刷新接入 `body` 全局循环

**Files:**
- Modify: `bionic_v0/src/bin/body.rs`

- [ ] **Step 1: 在 `run_global_refresh_cycle` 中插入调用顺序**  
  `refresh_global_reasoning_history` 之后、`write_snapshot` 之前，依次调用：  
  `refresh_probe_law(t)` -> `tick_probe_output_from_law(t)`。

- [ ] **Step 2: 保持串行拓扑不变**  
  不拆分新线程，不引入 request/response 动作接口，不绕开 orchestrator。

- [ ] **Step 3: 保持 body-observer 分离**  
  不在 body 增加 `println!/eprintln!`，调试可见性只走 snapshot/observer。

### Task 4: 让新子树进入 snapshot/observer（研究可见，不反向污染 body）

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: snapshot 扩展 `probe_law` 最小字段**  
  仅包含阶段、核心强度、时间戳与必要历史摘要；不记录世界内容标签。

- [ ] **Step 2: observer 新增 `prb:` 数值行**  
  只渲染数值与相位码，不输出人类意义解释。

- [ ] **Step 3: bump `SNAPSHOT_VERSION` 并补充兼容注释**

---

## Chunk 3: Contract Tests For “Truly Live” Probe

### Task 5: 新增主动探针进入 live body 的合同测试

**Files:**
- Create: `bionic_v0/tests/live_probe_entry.rs`
- Modify: `bionic_v0/tests/action_output_substrate.rs`（如需补充回归）
- Modify: `bionic_v0/tests/action_consequence_loop.rs`（如需补充回归）

- [ ] **Step 1: RED — 无外部命令情况下可产生 write epoch**  
  场景：仅推进 body 相关刷新（不手动调用 `apply_action_output_event`），断言 `n_write_epochs > 0`。

- [ ] **Step 2: RED — 结构抑制占优时探针会衰减/停出**  
  场景：构造连续抑制条件，断言输出相位转入 `Decaying/Idle`。

- [ ] **Step 3: RED — 与现有后果窗口/时间桥可闭合**  
  场景：probe 写出后注入 ingress 变化，断言可形成合法 pair（不校验结果意义）。

- [ ] **Step 4: RED — 边界防回退**  
  静态检查：新增代码不包含目标范围字段、结果类别字段、语义资格门关键词。

- [ ] **Step 5: GREEN — 逐项实现后跑全量相关测试**  
  Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml live_probe_entry action_output action_consequence -- --nocapture`

- [ ] **Step 6: 回归保护**  
  Run: `cargo test --manifest-path /Users/joe/Documents/unnamed/bionic_v0/Cargo.toml -- --nocapture`

---

## Acceptance Gates

- `body` 连续运行时，`action_output.n_write_epochs` 不再长期为 0。
- probe 写出后，`action_consequence` 可通过现有窗口与时间桥形成配对记录。
- 新增状态/测试未引入固定目标范围、结果分类、命名资格门。
- 主动探针阶段转移由结构关系驱动（历史相对更新），不是单阈值硬开关。
- body 无人类可读输出，observer/snapshot 可外部观测。

---

## Execution Order (Strict)

1. Chunk 1 Task 1
2. Chunk 1 Task 2
3. Chunk 2 Task 3
4. Chunk 2 Task 4
5. Chunk 3 Task 5

