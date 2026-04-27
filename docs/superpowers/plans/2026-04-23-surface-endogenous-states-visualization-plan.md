# Surface Endogenous States Visualization Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展现有 observer-side visual surface，将内生活动状态（探针强度、惊讶、注意力焦点、关系强度）以纯数值投射方式渲染为视觉表达

**Architecture:** 基于现有 surface_projection.rs/surface_render.rs，扩展 Glyph 结构添加数值场，在渲染层实现多通道颜色合成。遵循"颜色不是语义字典"原则——颜色由多个数值场共同影响，呈现为连续渐变而非离散语义标签。

**Tech Stack:** Rust, 现有 bionic_v0 substrate, snapshot 二进制

**Based on:**
- docs/superpowers/specs/2026-04-20-endogenous-activity-self-regulation-laws-design.md
- docs/superpowers/specs/2026-04-23-predictive-visual-surface-design.md

---

## 背景说明

当前 surface_projection 仅投射对象的空间信息（位置、范围、注意力份额、稳定度暗示）。根据 endogenous activity laws，系统内部存在以下可外显的活动状态：

1. **Probe Law 探针活动** — intensity/suppression 比率驱动探针阶段
2. **Global Surprise 全局惊讶** — 预测误差累积
3. **Attention Focus 注意力焦点** — routing_focus_slot_ids
4. **Relation Instability 关系不稳定** — 边 surprise 强度

这些是纯数值事实，可以通过视觉通道投射让外部观察者感知。

---

## 文件结构

```
bionic_v0/
├── src/
│   ├── surface_projection.rs  # 扩展 Glyph 结构 + 关系/全局层
│   ├── surface_render.rs      # 扩展多通道颜色合成
│   ├── surface_history.rs      # 可能需要扩展轨迹类型
│   └── snapshot.rs             # 确认已有字段存在
├── tests/
│   ├── surface_projection.rs   # 扩展测试
│   └── surface_render.rs       # 扩展测试
```

---

## Chunk 1: 扩展 Snapshot 字段读取

**目标:** 确认 snapshot 中已有内省状态字段，识别需要新增的读取

- [ ] **Step 1: 检查 SnapshotSlot 的 high_layer 字段**

读取 snapshot.rs 约 311-340 行，确认 SnapshotSlot 结构

```bash
# 查看 SnapshotSlot 结构
grep -n "pub struct SnapshotSlot" bionic_v0/src/snapshot.rs
# 查看 ObjectHighLayerState
grep -n "pub struct ObjectHighLayerState" bionic_v0/src/object_high_layer.rs
```

- [ ] **Step 2: 检查 GlobalReasoningState 字段**

```bash
grep -n "pub struct GlobalReasoningState" bionic_v0/src/viewport.rs
grep -n "pub struct GlobalAttentionState" bionic_v0/src/viewport.rs
grep -n "pub struct GlobalRelationState" bionic_v0/src/viewport.rs
```

- [ ] **Step 3: 检查 ProbeLawState 字段**

```bash
grep -n "pub struct ProbeLawState" bionic_v0/src/probe_law.rs
```

- [ ] **Step 4: 检查 ActionOutput 字段**

```bash
grep -n "pub struct ActionOutputState" bionic_v0/src/action_output.rs
```

- [ ] **Step 5: 确认 snapshot 序列化包含这些字段**

读取 snapshot.rs 确认 GlobalReasoningState, GlobalAttentionState, GlobalRelationState, ProbeLawState 是否已序列化到 SnapshotData

---

## Chunk 2: 扩展 SurfaceObjectGlyph 结构

**目标:** 在 surface_projection.rs 中添加新的数值场

- [ ] **Step 1: 扩展 SurfaceObjectGlyph 结构**

修改 bionic_v0/src/surface_projection.rs，添加以下数值场：

```rust
#[derive(Clone, Copy, Debug)]
pub struct SurfaceObjectGlyph {
    pub slot_id: usize,
    pub center_x: f64,
    pub center_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub attention_share: f64,
    pub stability_hint: f64,
    // === 新增内生活动数值场 ===
    /// 预测误差 hint → R 通道，数值范围 [0,1]
    pub err_hint: f64,
    /// 局部惊讶 hint → G 通道，数值范围 [0,1]
    pub surprise_hint: f64,
    /// 注意力偏向强度 → B 通道，数值范围 [0,1]
    pub attention_bias_hint: f64,
    /// 动作输出强度 → 团块亮度乘数，数值范围 [0,1]
    pub action_intensity_hint: f64,
}
```

- [ ] **Step 2: 更新 slot_to_glyph 函数**

修改 slot_to_glyph 函数，从 SnapshotSlot 中读取新增字段：

```rust
fn slot_to_glyph(slot_id: usize, slot: &SnapshotSlot, sp: SnapshotSpatial) -> SurfaceObjectGlyph {
    let stability_hint = compute_stability_hint(slot);
    let (err_hint, surprise_hint, attention_bias_hint) = compute_endogenous_hints(slot);

    SurfaceObjectGlyph {
        slot_id,
        center_x: sp.pos_x,
        center_y: sp.pos_y,
        radius_x: sp.span_x.max(0.0),
        radius_y: sp.span_y.max(0.0),
        attention_share: slot.attention_share,
        stability_hint,
        err_hint,
        surprise_hint,
        attention_bias_hint: slot.high_layer.as_ref()
            .map(|hl| hl.attention_bias)
            .unwrap_or(0.0),
        action_intensity_hint: 0.0, // 后续从全局状态填充
    }
}
```

- [ ] **Step 3: 新增 compute_endogenous_hints 函数**

```rust
fn compute_endogenous_hints(slot: &SnapshotSlot) -> (f64, f64, f64) {
    let mut err_hint = 0.0;
    let mut surprise_hint = 0.0;
    let mut attention_bias_hint = 0.0;

    if let Some(ref hl) = slot.high_layer {
        err_hint = hl.object_err.clamp(0.0, 1.0);
        surprise_hint = hl.predicted_surprise.clamp(0.0, 1.0);
        attention_bias_hint = hl.attention_bias.clamp(0.0, 1.0);
    }

    if let Some(ref fusion) = slot.fusion {
        err_hint = err_hint.max(fusion.fused_err.clamp(0.0, 1.0));
        surprise_hint = surprise_hint.max(fusion.fused_surprise.clamp(0.0, 1.0));
    }

    (err_hint, surprise_hint, attention_bias_hint)
}
```

- [ ] **Step 4: 扩展 SurfaceFrame 添加全局状态字段**

```rust
#[derive(Debug)]
pub struct SurfaceFrame {
    pub objects: Vec<SurfaceObjectGlyph>,
    pub relations: Vec<SurfaceRelationGlyph>,  // 新增
    pub global_probe_intensity: f64,           // 探针强度
    pub global_surprise_load: f64,              // 全局惊讶负载
    pub focus_slot_id: i8,                     // 焦点槽位 ID
}

#[derive(Clone, Copy, Debug)]
pub struct SurfaceRelationGlyph {
    pub source_slot_id: usize,
    pub target_slot_id: usize,
    pub strength: f64,
    pub surprise: f64,
}
```

- [ ] **Step 5: 更新 project_surface_frame 函数**

```rust
pub fn project_surface_frame(snap: &SnapshotData) -> SurfaceFrame {
    let mut objects = Vec::new();

    // 现有对象投影
    for (slot_id, slot) in snap.slots.iter().enumerate() {
        if !eligible_slot(slot) {
            continue;
        }
        if let Some(sp) = slot.spatial {
            if sp.valid {
                let glyph = slot_to_glyph(slot_id, slot, sp);
                objects.push(glyph);
            }
        }
    }

    // 新增：关系边投影
    let relations = project_relations(&snap.global_relations, &snap.slots);

    // 新增：全局状态投影
    let focus_slot_id = snap.global_reasoning.routing_focus_slot_ids.first().copied().unwrap_or(-1);

    // 探针强度和全局惊讶从 global_reasoning 读取
    let probe_law = snap.global_reasoning_memory.probe_intensity; // 或从 probe_law 读取
    let global_surprise = snap.global_reasoning.global_surprise_load;

    SurfaceFrame {
        objects,
        relations,
        global_probe_intensity: probe_law,
        global_surprise_load: global_surprise,
        focus_slot_id,
    }
}
```

- [ ] **Step 6: 新增 project_relations 函数**

```rust
fn project_relations(rels: &GlobalRelationState, slots: &[SnapshotSlot]) -> Vec<SurfaceRelationGlyph> {
    let mut glyphs = Vec::new();

    for edge in &rels.edges {
        if !edge.active {
            continue;
        }

        // 检查源和目标槽位是否活跃
        if edge.source_slot_id as usize >= slots.len()
            || edge.target_slot_id as usize >= slots.len() {
            continue;
        }

        let source_active = slots[edge.source_slot_id as usize].active;
        let target_active = slots[edge.target_slot_id as usize].active;

        if !source_active || !target_active {
            continue;
        }

        glyphs.push(SurfaceRelationGlyph {
            source_slot_id: edge.source_slot_id as usize,
            target_slot_id: edge.target_slot_id as usize,
            strength: edge.pred.strength.clamp(0.0, 1.0),
            surprise: edge.pred.relation_surprise.clamp(0.0, 1.0),
        });
    }

    glyphs
}
```

- [ ] **Step 7: 运行测试验证**

```bash
cd bionic_v0 && cargo test surface_projection -- --nocapture
```

预期：编译通过，测试通过

- [ ] **Step 8: Commit**

```bash
git add bionic_v0/src/surface_projection.rs
git commit -m "feat: extend SurfaceObjectGlyph with endogenous state hints"
```

---

## Chunk 3: 扩展 Surface Render 多通道颜色合成

**目标:** 在 surface_render.rs 中实现多通道颜色合成，将数值场映射为视觉表达

- [ ] **Step 1: 修改 fill_object_blob 函数支持多通道**

修改 bionic_v0/src/surface_render.rs，扩展 fill_object_blob：

```rust
fn fill_object_blob(
    buffer: &mut [[u8; 4]],
    obj: &crate::surface_projection::SurfaceObjectGlyph,
    width: usize,
    height: usize,
) {
    let cx = obj.center_x * width as f64;
    let cy = obj.center_y * height as f64;
    let rx = obj.radius_x * width as f64;
    let ry = obj.radius_y * height as f64;

    let min_x = (cx - rx).max(0.0) as usize;
    let max_x = (cx + rx).min(width as f64 - 1.0) as usize;
    let min_y = (cy - ry).max(0.0) as usize;
    let max_y = (cy + ry).min(height as f64 - 1.0) as usize;

    // 基础亮度：由 attention_share 和 stability_hint 决定
    let base_intensity = ((0.35 + 0.65 * obj.stability_hint.clamp(0.0, 1.0))
        * (96.0 + 159.0 * obj.attention_share.clamp(0.0, 1.0))) as u8;

    // 多通道颜色合成
    // R = err_hint (预测误差) - 高误差偏红
    // G = surprise_hint (惊讶) + attention_bias_hint - 高惊讶/高注意力偏绿
    // B = attention_bias_hint (注意力偏向)
    // 亮度乘数 = action_intensity_hint

    let r = (obj.err_hint.clamp(0.0, 1.0) * 255.0) as u8;
    let g = ((obj.surprise_hint + obj.attention_bias_hint * 0.5).clamp(0.0, 1.0) * 255.0) as u8;
    let b = (obj.attention_bias_hint.clamp(0.0, 1.0) * 255.0) as u8;

    let intensity_multiplier = 0.5 + 0.5 * obj.action_intensity_hint.clamp(0.0, 1.0);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = (x as f64 - cx) / rx.max(1.0);
            let dy = (y as f64 - cy) / ry.max(1.0);
            if dx * dx + dy * dy <= 1.0 {
                let idx = y * width + x;
                if idx < buffer.len() {
                    // 颜色混合：基础灰度 + 通道着色
                    let base = (base_intensity as f64 * intensity_multiplier) as u8;
                    buffer[idx][0] = buffer[idx][0].max((r * 2 / 3 + base * 1 / 3) as u8);
                    buffer[idx][1] = buffer[idx][1].max((g * 2 / 3 + base * 1 / 3) as u8);
                    buffer[idx][2] = buffer[idx][2].max((b * 2 / 3 + base * 1 / 3) as u8);
                    buffer[idx][3] = 255;
                }
            }
        }
    }
}
```

- [ ] **Step 2: 新增渲染关系边的函数**

```rust
fn render_relation_glyph(
    buffer: &mut [[u8; 4]],
    rel: &crate::surface_projection::SurfaceRelationGlyph,
    objects: &[crate::surface_projection::SurfaceObjectGlyph],
    width: usize,
    height: usize,
) {
    // 查找源和目标对象位置
    let source = objects.iter().find(|o| o.slot_id == rel.source_slot_id);
    let target = objects.iter().find(|o| o.slot_id == rel.target_slot_id);

    let (Some(src), Some(tgt)) = (source, target) else {
        return;
    };

    let x0 = (src.center_x * width as f64) as isize;
    let y0 = (src.center_y * height as f64) as isize;
    let x1 = (tgt.center_x * width as f64) as isize;
    let y1 = (tgt.center_y * height as f64) as isize;

    // 简单直线绘制 (Bresenham)
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut x = x0;
    let mut y = y0;
    let mut continue_drawing = true;

    while continue_drawing {
        if x >= 0 && x < width as isize && y >= 0 && y < height as isize {
            let idx = y as usize * width + x as usize;
            if idx < buffer.len() {
                // 关系颜色：strength 控制透明度，surprise 控制 R 分量
                let alpha = (rel.strength * 128.0) as u8;
                let surprise_r = (rel.surprise * 192.0) as u8;
                buffer[idx][0] = buffer[idx][0].max(surprise_r);
                buffer[idx][1] = buffer[idx][1].max(alpha / 2);
                buffer[idx][2] = buffer[idx][2].max(alpha / 2);
                buffer[idx][3] = buffer[idx][3].max(alpha);
            }
        }

        if x == x1 && y == y1 {
            continue_drawing = false;
        } else {
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}
```

- [ ] **Step 3: 更新 render_surface_frame 函数**

```rust
pub fn render_surface_frame(
    frame: &SurfaceFrame,
    trails: &[SlotTrail],
    config: RenderConfig,
) -> Vec<[u8; 4]> {
    let total_pixels = config.width * config.height;
    let mut buffer = vec![[0u8; 4]; total_pixels];

    // 绘制拖尾层（背景）
    for trail in trails {
        for point in &trail.points {
            stamp_trail_point(&mut buffer, point, config.width, config.height);
        }
    }

    // 绘制关系边层
    for rel in &frame.relations {
        render_relation_glyph(&mut buffer, rel, &frame.objects, config.width, config.height);
    }

    // 绘制对象团块层（顶层）
    for obj in &frame.objects {
        fill_object_blob(&mut buffer, obj, config.width, config.height);
    }

    buffer
}
```

- [ ] **Step 4: 新增全局状态可视化（背景层）**

```rust
/// 在 render_surface_frame 开头添加全局状态背景
fn apply_global_state_background(
    buffer: &mut [[u8; 4]],
    frame: &SurfaceFrame,
    width: usize,
    height: usize,
) {
    // 全局探针强度决定背景微弱亮度
    let bg_intensity = (frame.global_probe_intensity * 24.0) as u8;

    // 全局惊讶负载决定背景 R 分量
    let bg_r = (frame.global_surprise_load * 48.0) as u8;

    for idx in 0..buffer.len() {
        buffer[idx][0] = buffer[idx][0].max(bg_r);
        buffer[idx][1] = buffer[idx][1].max(bg_intensity / 2);
        buffer[idx][2] = buffer[idx][2].max(bg_intensity / 2);
    }
}
```

- [ ] **Step 5: 在 render_surface_frame 中调用**

```rust
pub fn render_surface_frame(
    frame: &SurfaceFrame,
    trails: &[SlotTrail],
    config: RenderConfig,
) -> Vec<[u8; 4]> {
    let total_pixels = config.width * config.height;
    let mut buffer = vec![[0u8; 4]; total_pixels];

    // 全局状态背景层
    apply_global_state_background(&mut buffer, frame, config.width, config.height);

    // ... 其余层级
}
```

- [ ] **Step 6: 运行测试验证**

```bash
cd bionic_v0 && cargo test surface_render -- --nocapture
```

预期：编译通过，测试通过

- [ ] **Step 7: Commit**

```bash
git add bionic_v0/src/surface_render.rs
git commit -m "feat: extend surface render with multi-channel color synthesis"
```

---

## Chunk 4: 扩展 Observer Surface 集成

**目标:** 确保 observer_surface.rs 正确使用扩展后的 SurfaceFrame

- [ ] **Step 1: 检查现有 observer_surface.rs**

```bash
cat bionic_v0/src/bin/observer_surface.rs
```

- [ ] **Step 2: 确认它调用 project_surface_frame**

读取现有代码，确认调用流程。如果已有正确流程，则此 chunk 可跳过。

- [ ] **Step 3: 如需要，更新调用以传递新字段**

如果 observer_surface.rs 需要访问新增的全局状态，添加相应读取逻辑。

---

## Chunk 5: 添加测试

**目标:** 确保扩展功能有充分测试覆盖

- [ ] **Step 1: 添加 surface_projection 单元测试**

在 tests/surface_projection.rs 添加：

```rust
#[test]
fn test_endogenous_hints_computation() {
    // 测试 compute_endogenous_hints 函数
    // 验证 err_hint, surprise_hint, attention_bias_hint 正确计算
}
```

- [ ] **Step 2: 添加 surface_render 单元测试**

在 tests/surface_render.rs 添加：

```rust
#[test]
fn test_multi_channel_color_synthesis() {
    // 测试多通道颜色合成
    // 验证 R/G/B 通道正确映射
}
```

- [ ] **Step 3: 运行所有测试**

```bash
cd bionic_v0 && cargo test -- --nocapture
```

预期：所有测试通过

- [ ] **Step 4: Commit**

```bash
git add bionic_v0/tests/
git commit -m "test: add endogenous state visualization tests"
```

---

## 设计原则核对

| 原则 | 符合性 |
|------|--------|
| 纯数值投射 | ✅ 所有字段来自 snapshot 数值，无语义标签 |
| 颜色不是语义字典 | ✅ 颜色由多个数值场共同影响（err+surprise+attention），连续渐变 |
| Observer-side 实现 | ✅ 仅扩展 projection/render，不修改 body |
| 不回写 body | ✅ 渲染层只读 snapshot，不修改任何 body 状态 |
| 不使用情绪词汇 | ✅ 所有新字段使用 _hint 后缀，表示数值暗示而非语义 |
| 不命名 meaning/identity | ✅ 无此类字段 |
| 性能约束 | ✅ 扩展保持 O(n) 复杂度，n = 活跃对象数 + 边数 |

---

## 验收标准

- [ ] SurfaceObjectGlyph 包含 err_hint, surprise_hint, attention_bias_hint, action_intensity_hint
- [ ] SurfaceFrame 包含 relations, global_probe_intensity, global_surprise_load, focus_slot_id
- [ ] 渲染器支持多通道颜色合成：R=err, G=surprise+attention, B=attention
- [ ] 渲染器支持关系边绘制
- [ ] 所有测试通过
- [ ] 不违反 predictive visual surface spec 禁令
- [ ] 不违反 endogenous activity laws 开发准入规则

---

## 依赖关系

- Chunk 1: 基础确认（可快速验证）
- Chunk 2: 依赖 Chunk 1 的字段确认结果
- Chunk 3: 依赖 Chunk 2 的 Glyph 扩展
- Chunk 4: 依赖 Chunk 2+3
- Chunk 5: 依赖所有 Chunk 完成

**Plan complete.** Ready to execute?

