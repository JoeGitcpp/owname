# 2026-05-15 Owname 接手原则与轻量工作流

## 当前状态

- 仓库：`/Users/joe/unnamed`
- 分支：`main`
- 本地状态：干净
- 本次未启动 Symphony agent，未启动后台服务，未写实现代码。

## 必读入口

- `AGENTS.md`
- `consciousness-stream-architecture.md`
- `tier0-base-elements.md`
- `docs/axioms/2026-04-20-new-axioms-v1.md`
- `docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md`
- `docs/superpowers/specs/2026-04-20-language-binding-layer-design.md`
- `docs/superpowers/specs/2026-04-29-subconscious-conscious-language-boundary-design.md`
- `docs/superpowers/specs/2026-05-05-live-io-preflight-life-sign-contract-design.md`

## 产品原则

- Owname 是 silicon-life substrate，不是 request/response 软件。
- 研究观察和 AI body 必须分离；body 侧不得输出人类可读解释，观察属于 observer。
- tier 0 是共享坐标基底，不是语言内容；对象先于命名，参数是时间曲线，不是静态标签。
- 主动性、探索、模仿、自调节和自修复只能预设学习法则、body 法则、更新约束和安全边界；对象范围、目标范围、重要性图必须是学习形成和可修订的。

## 设计原则

- 禁止重新引入 `semantic_readiness`、`symbolizer`、`concept_id`、命名资源门、稳定对象到离散概念 ID 的映射。
- “潜意识”与“表意识”只能作为外部设计隐喻，不能落成内部模块或 ontology。
- 高风险术语必须写边界说明：`semantic`、`concept`、`token`、`label`、`name`、`word`、`identity`、`personality` 等。
- `bionic_v0/experimental/` 是隔离考古区，不得恢复到 active tests 或 Cargo。

## 技术原则

- 保留核心 substrate：perception ingress、object slots/fusion/ambient channel、viewport substrate、snapshot、observer。
- 新模块必须通过 frozen specs 的 admission rules；不能用字段或模块提前规定系统“真正学习什么”。
- 测试应优先保护架构边界：body 无人类输出、观察在 observer、无命名门、无概念分配器、无 token ontology。
- 如需数值参数，只能作为理论的实现阴影，不能反过来定义理论。

## 轻量 agent 工作流

### CEO / Product

职责：判断请求是否符合 silicon-life substrate 方向，拒绝 request/response、固定人格、固定语义门、固定世界内容。

入口提示词：

```text
请在 /Users/joe/unnamed 做 Owname 产品审查。先读 AGENTS.md 和其中列出的必读 specs。不要写代码。判断 issue 是否符合 silicon-life substrate：body/observer 分离、对象先于命名、学习边界可修订、不得引入 request/response 或固定语义/人格结构。输出中文结论、违反风险、推荐下一状态。
```

### CTO / Architecture

职责：检查模块边界、数据流和 frozen specs 一致性；确认不恢复 quarantine 架构。

入口提示词：

```text
请在 /Users/joe/unnamed 做 Owname 架构审查。先读 AGENTS.md、architecture cleanup spec、language binding spec 和相关代码路径。不要写代码。沿 body、snapshot、observer、viewport、binding/event 数据流检查方案是否引入 semantic_readiness、symbolizer、concept_id、命名门或 token ontology。输出中文架构结论和必须修改的边界。
```

### Principle Guardian

职责：专门拦截术语和原则滑坡。

入口提示词：

```text
请在 /Users/joe/unnamed 做 Owname 原则守卫审查。先读 AGENTS.md 的 Rejected Architecture、Subconscious/Conscious Boundary、High-Risk Terms。不要写代码。检查 proposal 或 diff 中所有 high-risk terms，并逐项说明它们是外部描述、工程索引还是非法 ontology。不能回答清楚的，要求回到设计阶段。
```

### Builder

职责：只在产品、架构、原则都通过后实现；必须先写计划和测试。

入口提示词：

```text
请在 /Users/joe/unnamed 实现已通过审查的 Owname 任务。先读 AGENTS.md 和相关 frozen spec，再写简短实现计划。必须先补失败测试，再写代码。不得修改 quarantine 文件，不得引入 body 侧人类可读输出，不得新增 semantic/concept/name/token ontology。完成后运行相关 cargo test，并汇报证据。
```

### QA

职责：验证实现没有破坏 substrate 和冻结边界。

入口提示词：

```text
请在 /Users/joe/unnamed 做 Owname QA。先读 AGENTS.md 和本任务关联 specs。不要修代码，除非明确要求。运行相关 cargo test，重点检查 body_no_human_output、architecture_contraction、language_binding、live_io_preflight、quarantine 不复活等边界。输出中文通过/失败、关键错误和剩余风险。
```

## 工作入口建议

- 新需求先进入 CEO / Product。
- 涉及语言、命名、概念、人格、意识术语时，必须经过 Principle Guardian。
- 实现前必须有明确 spec 或 plan；没有计划时不进入 Builder。
- 不启动后台长驻 agent；每个任务单独显式审查、显式执行、显式验证。
