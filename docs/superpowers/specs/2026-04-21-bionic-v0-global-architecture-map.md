# bionic_v0 全局架构版图（2026-04-21）

> 状态：工作基线（用于对齐“已落地 / 未落地 / 优先级 / 缺口”）

## 0. 边界说明

本文理论描述尽量只使用：对象、关系、时间结构、共享视口、外部模式、写出历元、后果窗口、快照、观察口。

本文出现的工程文件名（如 `language_binding.rs`、`action_output.rs`、`viewport.rs`）全部是**工程索引名，不是内部本体**。

## 1. 当前总图（按层级）

```text
外部模式(感知)
  -> 共享视口(对象 / 关系 / 时间结构)
  -> 快照
  -> 观察口

共享视口
  -> 写出历元
  -> 后果窗口
  -> 快照
  -> 观察口

外部模式(符号/声音)
  -> 绑定轨迹(工程索引: language_binding.rs)
  -> 共享视口
```

## 2. 已落地层级

| 层级 | 当前状态 | 工程落点（均为工程索引名，不是内部本体） | 证据 |
|---|---|---|---|
| 外部模式 -> 共享视口（对象/关系/时间结构） | 已落地（主循环在跑） | `adapter.rs`、`viewport.rs`、`predictor.rs`、`phot_predictor.rs`、`object_high_layer.rs`、`bin/body.rs` | 主循环包含全局刷新链；适配器持续写入共享视口 |
| 共享视口 -> 快照 -> 观察口 | 已落地 | `snapshot.rs`、`bin/observer.rs` | 快照序列化和观察口渲染稳定存在 |
| 写出历元（结构本身） | 已落地（结构层） | `action_output.rs`、`viewport.rs`、`snapshot.rs`、`observer.rs` | `action_output_substrate` 测试通过 |
| 后果窗口（结构本身） | 已落地（结构层） | `action_consequence.rs`、`viewport.rs`、`snapshot.rs`、`observer.rs` | `action_consequence_loop` 测试通过 |
| 清理边界（去除被拒绝层） | 已落地 | `cleanup_hygiene.rs`、`ordered_global_refresh.rs` | 相关防回退测试通过 |

## 3. 半落地 / 未落地层级

| 层级 | 状态 | 说明 |
|---|---|---|
| 共享视口 -> 写出历元（活态触发） | 未落地 | `src/` 内没有任何运行态调用 `apply_action_output_event(...)`，写出历元在真实运行中缺少触发源 |
| 写出历元 -> 后果窗口（活态配对闭环） | 半落地 | 自动配对函数存在并被调用，但上游写出历元缺少主循环触发，闭环仍不完整 |
| 外部模式 -> 绑定轨迹 -> 共享视口（运行态） | 半落地 | 绑定轨迹结构存在，但运行链中缺少调用 `apply_language_binding_event(...)` |
| 后果窗口保留策略（按开闭与引用优先淘汰） | 部分未落地 | 当前实现主要按 Vec 头部淘汰；与冻结规范中“优先级淘汰顺序”仍有差距 |
| 外部模式短时保持/比较/逼近（后续法则层） | 未落地 | 冻结规范已定义方向，但工程中尚无对应运行层 |

## 4. 优先级顺序（建议执行）

1. P0：补齐“共享视口 -> 写出历元”的真实触发路径（先让写出历元在主循环稳定出现）。
2. P1：在真实运行中完成“写出历元 -> 后果窗口”闭环验证（不是只在测试里成立）。
3. P2：把“外部模式 -> 绑定轨迹”从测试路径推进到运行路径，但保持绑定轨迹仅作绑定，不改写对象/关系本体。
4. P3：对齐后果窗口历史保留策略与冻结规范（淘汰顺序、引用完整性边界）。
5. P4：再进入“外部模式短时保持/比较/逼近”等后续层。

## 5. 当前已知缺口（可直接跟踪）

1. 写出历元无运行态触发源：`rg` 结果显示 `src/` 中 `apply_action_output_event(...)` 仅定义未被调用。
2. 绑定轨迹无运行态入口：`apply_language_binding_event(...)` 目前只在测试中出现。
3. `probe` 线路尚未进仓：`src/` 内不存在 `tick_probe_output` / `tick_consequence_ingress` / `PROBE_*` 常量。
4. 后果窗口目前主要由 PHOT 路径推动，跨更多外部模式的统一入口尚未完成。
5. 后果窗口淘汰策略与冻结规范存在实现差距（当前以工程上界为主，优先级淘汰细则未完全落地）。

## 6. 当前基线测试（2026-04-21）

已执行并通过：
- `action_consequence_loop`（45 passed）
- `action_output_substrate`（9 passed）
- `language_binding`（7 passed）
- `cleanup_hygiene`（3 passed）
- `ordered_global_refresh`（5 passed）
- `body_no_human_output`（1 passed）

这组结果说明：结构底座已经可测成立；当前主要缺口在“运行态接通顺序”，而不是“结构是否存在”。
