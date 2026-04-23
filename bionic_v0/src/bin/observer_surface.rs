use std::env;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bionic_v0::snapshot::read_snapshot;
use bionic_v0::surface_history::SurfaceHistory;
use bionic_v0::surface_projection::project_surface_frame;
use bionic_v0::surface_render::{render_surface_frame, RenderConfig};

use pixels::Pixels;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const INTERNAL_WIDTH: usize = 160;
const INTERNAL_HEIGHT: usize = 90;
const WINDOW_SCALE: u32 = 6;
const POLL_INTERVAL_MS: u64 = 33;

fn main() {
    let args: Vec<String> = env::args().collect();
    let runtime_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        eprintln!("Usage: observer_surface <runtime_dir>");
        std::process::exit(1);
    };

    let event_loop = EventLoop::new();

    let window_size = winit::dpi::PhysicalSize::new(
        INTERNAL_WIDTH as u32 * WINDOW_SCALE,
        INTERNAL_HEIGHT as u32 * WINDOW_SCALE,
    );
    let window = WindowBuilder::new()
        .with_title("Predictive Visual Surface")
        .with_resizable(false)
        .with_inner_size(window_size)
        .build(&event_loop)
        .unwrap();

    let surface_texture = pixels::SurfaceTexture::new(
        INTERNAL_WIDTH as u32,
        INTERNAL_HEIGHT as u32,
        &window,
    );
    let mut pixels = Pixels::new(
        INTERNAL_WIDTH as u32,
        INTERNAL_HEIGHT as u32,
        surface_texture,
    ).unwrap();

    let mut history = SurfaceHistory::new(16);
    let mut last_poll = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawRequested(_) => {
                let now = Instant::now();
                if now.duration_since(last_poll) >= Duration::from_millis(POLL_INTERVAL_MS) {
                    last_poll = now;

                    let snapshot_path = runtime_dir.join("snapshot.bin");
                    let frame = if let Ok(snap) = read_snapshot(&snapshot_path) {
                        project_surface_frame(&snap)
                    } else {
                        bionic_v0::surface_projection::SurfaceFrame { objects: Vec::new() }
                    };

                    history.update(&frame);
                    let trails = history.render_trails();

                    let config = RenderConfig::new(INTERNAL_WIDTH, INTERNAL_HEIGHT);
                    let img = render_surface_frame(&frame, &trails, config);

                    let texture = pixels.frame_mut();
                    for (dest, src) in texture.chunks_exact_mut(4).zip(img.iter()) {
                        dest[0] = src[0];
                        dest[1] = src[1];
                        dest[2] = src[2];
                        dest[3] = src[3];
                    }
                    let _ = pixels.render();
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state == ElementState::Pressed && input.virtual_keycode == Some(VirtualKeyCode::Escape) {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
