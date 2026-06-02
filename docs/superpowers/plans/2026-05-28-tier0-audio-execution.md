# Tier 0 音频 — 执行方案（阶段 A）

> 日期：2026-05-28
> 依据：docs/superpowers/specs/2026-05-28-tier0-audio-raw-waveform-architecture-design.md
> 范围：阶段 A — audio_raw 基础通道
> 状态：待执行

---

## 执行摘要

新增 `AudioAdapter`，将麦克风原始波形（f32 采样点）写入视口 ring buffer。不替代当前 VIB 7-band——作为**并行通道**同时存在。

- 新增 1 个文件 + 改动 4 个文件
- 预计 ~200 行代码变更
- 阶段 A 不做任何 predictor，纯数据搬运
- 当前 VIB + vocal_reflex + vocal_target 行为完全不受影响

---

## 执行步骤

### Step A1 · 新增 audio_raw 常量 + 轴定义

**文件**：`bionic_v0/src/lib.rs`

```rust
/// audio_raw 环形缓冲区容量
/// 48,000 Hz × 60 秒 = 2,880,000 个 f32
pub const AUDIO_RAW_CAPACITY: usize = 48_000 * 60;

/// 批量写入间隔：每 10ms 写入一次（48,000 × 0.01 = 480 个采样点）
pub const AUDIO_RAW_BATCH_SAMPLES: usize = 480;
pub const AUDIO_RAW_BATCH_INTERVAL_US: u64 = 10_000;
```

### Step A2 · 新增 AudioRawBuffer

**文件**：`bionic_v0/src/audio_raw.rs`（新文件）

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

/// 环形缓冲区 — 存储原始音频波形
pub struct AudioRawBuffer {
    /// 采样点环形存储
    samples: RwLock<Vec<f32>>,
    /// 当前写入位置（原子，无锁读）
    write_pos: AtomicU64,
    /// 已写入的总采样点数
    total_samples: AtomicU64,
    /// 最后一个采样点的时钟（us）
    last_sample_clock_us: AtomicU64,
}

impl AudioRawBuffer {
    pub fn new() -> Self {
        Self {
            samples: RwLock::new(vec![0.0_f32; AUDIO_RAW_CAPACITY]),
            write_pos: AtomicU64::new(0),
            total_samples: AtomicU64::new(0),
            last_sample_clock_us: AtomicU64::new(0),
        }
    }

    /// 批量写入采样点（由 AudioAdapter 调用）
    pub fn write_batch(&self, batch: &[f32], clock_us: u64) {
        let n = batch.len().min(AUDIO_RAW_CAPACITY);
        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let mut samples = self.samples.write().unwrap();

        if pos + n <= AUDIO_RAW_CAPACITY {
            samples[pos..pos + n].copy_from_slice(&batch[..n]);
        } else {
            // 环形回绕
            let first = AUDIO_RAW_CAPACITY - pos;
            samples[pos..].copy_from_slice(&batch[..first]);
            samples[..n - first].copy_from_slice(&batch[first..n]);
        }

        self.write_pos.store(((pos + n) % AUDIO_RAW_CAPACITY) as u64, Ordering::Relaxed);
        self.total_samples.fetch_add(n as u64, Ordering::Relaxed);
        self.last_sample_clock_us.store(clock_us, Ordering::Relaxed);
    }

    /// 读取最近的 N 个采样点（观察者/Predictor 使用）
    pub fn read_recent(&self, n: usize) -> Vec<f32> {
        let n = n.min(AUDIO_RAW_CAPACITY);
        let total = self.total_samples.load(Ordering::Relaxed) as usize;
        if total == 0 {
            return vec![0.0; n];
        }

        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let start = if total < n { 0 } else { (pos + AUDIO_RAW_CAPACITY - n) % AUDIO_RAW_CAPACITY };
        let samples = self.samples.read().unwrap();

        if start + n <= AUDIO_RAW_CAPACITY {
            samples[start..start + n].to_vec()
        } else {
            let mut v = Vec::with_capacity(n);
            let first = AUDIO_RAW_CAPACITY - start;
            v.extend_from_slice(&samples[start..]);
            v.extend_from_slice(&samples[..n - first]);
            v
        }
    }

    /// 单采样点读取（高频 Predictor 使用）
    pub fn read_sample_at(&self, offset_from_end: usize) -> f32 {
        let total = self.total_samples.load(Ordering::Relaxed) as usize;
        if total == 0 || offset_from_end >= total {
            return 0.0;
        }
        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let idx = (pos + AUDIO_RAW_CAPACITY - offset_from_end - 1) % AUDIO_RAW_CAPACITY;
        self.samples.read().unwrap()[idx]
    }
}
```

**验证**：
- 写入 480 个采样点 → 读取最近 480 个 → 数据一致
- 写入超过 60 秒 → 环形覆盖 → 读取最近 60 秒数据
- 多线程写 + 多线程读 → 无 panic（RwLock 保证）

### Step A3 · 新增 AudioAdapter

**文件**：`bionic_v0/src/audio_adapter.rs`（新文件）

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::audio_raw::AudioRawBuffer;
use crate::clock::now_us;
use crate::AUDIO_RAW_BATCH_SAMPLES;

/// AudioAdapter — 从系统默认麦克风读取原始波形，写入 AudioRawBuffer。
/// 不做 FFT、不做频段提取、不做任何预处理。
/// 与 VibAdapter 并行运行，各自独立。
pub struct AudioAdapter {
    _stream: cpal::Stream,
    fault: Arc<AtomicBool>,
}

impl AudioAdapter {
    pub fn new(audio_buf: Arc<AudioRawBuffer>) -> Self {
        let fault = Arc::new(AtomicBool::new(false));
        match Self::try_new_parts(audio_buf, fault.clone()) {
            Ok(stream) => Self { _stream: stream, fault },
            Err(_) => Self {
                _stream: /* 空的 cpal Stream 不可用。改为在 body.rs 里处理。 */
                fault,
            },
        }
    }

    fn try_new_parts(
        audio_buf: Arc<AudioRawBuffer>,
        fault: Arc<AtomicBool>,
    ) -> Result<cpal::Stream, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("no default audio input device")?;
        let config = device.default_input_config()?;
        let stream_config: StreamConfig = config.into();

        // 累加器：收集到 AUDIO_RAW_BATCH_SAMPLES 个后批量写入
        let accumulator = std::cell::RefCell::new(Vec::<f32>::with_capacity(AUDIO_RAW_BATCH_SAMPLES));

        let stream = device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut acc = accumulator.borrow_mut();
                acc.extend_from_slice(data);
                if acc.len() >= AUDIO_RAW_BATCH_SAMPLES {
                    let batch: Vec<f32> = acc.drain(..AUDIO_RAW_BATCH_SAMPLES).collect();
                    audio_buf.write_batch(&batch, now_us());
                }
            },
            move |_err| {
                fault.store(true, Ordering::Relaxed);
            },
            None,
        )?;

        Ok(stream)
    }
}
```

**注意**：`cpal::Stream` 不是 `Send`。如果 `AudioAdapter` 需要在注册表线程创建，需要把 stream 创建移到驱动线程内部。execution 时处理。

### Step A4 · Viewport 集成

**文件**：`bionic_v0/src/viewport.rs`

```rust
use crate::audio_raw::AudioRawBuffer;

pub struct Viewport {
    // ... 现有字段 ...

    /// Tier 0 原始音频波形缓冲区。阶段 A 只写入不读取（观察者可通过
    /// snapshot 查看）。阶段 B 起 audio predictor 从此读取。
    pub audio_raw: Arc<AudioRawBuffer>,
}
```

**初始化**：
```rust
// Viewport::new() 中
audio_raw: Arc::new(AudioRawBuffer::new()),
```

### Step A5 · Body 启动集成

**文件**：`bionic_v0/src/bin/body.rs`

```rust
use bionic_v0::audio_adapter::AudioAdapter;

// 在 adapter 注册阶段（现有 VibAdapter 注册的同一位置）：
let vp_audio = vp.clone();
let audio_buf = vp.audio_raw.clone();
std::thread::spawn(move || {
    let _adapter = AudioAdapter::new(audio_buf);
    // cpal stream 保持存活，持续回调
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }
    }
});
```

### Step A6 · Snapshot 观测支持

**文件**：`bionic_v0/src/snapshot.rs`

在 snapshot 中写入 audio_raw 的状态摘要（不写全部采样点——太大）：

```rust
// 写入
buf.extend_from_slice(&vp.audio_raw.total_samples.load(Ordering::Relaxed).to_le_bytes());
buf.extend_from_slice(&vp.audio_raw.last_sample_clock_us.load(Ordering::Relaxed).to_le_bytes());
// 不写实际波形数据（2.88M f32 = 11.5MB，太大）

// 读取
let audio_total_samples = rd_u64(bytes, cur);
let audio_last_clock_us = rd_u64(bytes, cur);
```

### Step A7 · Curriculum Report 观测

**文件**：`bionic_v0/src/bin/curriculum_report.rs`

在 `--live-human-nursery` 报告中增加：

```rust
println!("audio_raw_total_samples={}", snap.audio_total_samples);
println!("audio_raw_active={}", u8::from(audio_total_samples > 0 && audio_last_clock_us > now_us - 5_000_000));
```

---

## 影响范围

| 文件 | 改动类型 | 行数 |
|------|---------|------|
| `src/audio_raw.rs` | **新增** | ~80 行 |
| `src/audio_adapter.rs` | **新增** | ~60 行 |
| `src/lib.rs` | 新增常量 | ~5 行 |
| `src/viewport.rs` | 新增字段 | ~5 行 |
| `src/bin/body.rs` | 新增 adapter 启动 | ~15 行 |
| `src/snapshot.rs` | 新增摘要读写 | ~10 行 |
| `src/bin/curriculum_report.rs` | 新增报告行 | ~5 行 |
| `tests/audio_raw.rs` | **新增** | ~50 行 |

---

## 验证清单

### 编译验证
- [ ] `cargo build --release` 成功
- [ ] `cargo test` 全部通过（包括新的 audio_raw 测试）

### 功能验证
- [ ] Body 启动后 `audio_raw_total_samples > 0`（1 秒内应 > 48000）
- [ ] `audio_raw_active = 1`
- [ ] 现有 VIB 7-band 行为不受影响
- [ ] `vocal_reflex` 行为不受影响
- [ ] `vocal_target` 行为不受影响

### 性能验证
- [ ] Body 运行时 CPU 增量 < 5%（阶段 A 只做 memcpy，无计算）
- [ ] 内存增量 = 11.5MB（60s ring buffer）

---

## 阶段 B 入口（不在本次执行）

阶段 B 将在 `audio_raw` 上增加第一个 AudioWavePredictor，验证"predictor 能从原始波形学到频谱结构"。阶段 A 完成后 + 验证通过才进入。
