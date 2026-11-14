//! Drawing into windows (spec 14.24, decision 0106), by number, as the
//! interpreter and native code call it. A screen is a surface on a
//! window, made from the instance the program's device came from and
//! sized to the window at each frame. A frame is the surface's next
//! picture: what is drawn into it is recorded, and `show` sends it.

use crate::device::gpu;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

struct Screen {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

struct Frame {
    screen: i64,
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
}

#[derive(Default)]
struct Drawing {
    screens: HashMap<i64, Screen>,
    frames: HashMap<i64, Frame>,
    next: i64,
}

fn drawing() -> MutexGuard<'static, Drawing> {
    static DRAWING: OnceLock<Mutex<Drawing>> = OnceLock::new();
    DRAWING
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// A screen on window number `n` of `kavach_runtime_window`, now `size`
/// pixels: its number, or why it can't be drawn into. The surface is
/// made on the window's own thread, where every system gives its handle;
/// the device opens before, so the window's events don't wait for it.
pub fn screen_on(n: i64, size: (u32, u32)) -> Result<i64, String> {
    if gpu().is_none() {
        return Err("this machine has no GPU".into());
    }
    kavach_runtime_window::with_window(n, move |window| screen(window, size.0, size.1))
        .unwrap_or_else(|| Err("the window is closed".into()))
}

/// A screen on `window`, now `width` × `height` pixels: its number, or
/// why it can't be drawn into.
fn screen(
    window: impl Into<wgpu::SurfaceTarget<'static>>,
    width: u32,
    height: u32,
) -> Result<i64, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let surface = g
        .instance
        .create_surface(window)
        .map_err(|e| e.to_string())?;
    let caps = surface.get_capabilities(&g.adapter);
    // The colors programs give are in sRGB, as screens show them.
    let format = (caps.formats.iter().copied())
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| caps.formats.first().copied())
        .ok_or("the GPU can't draw into this window")?;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: width.max(1),
        height: height.max(1),
        // Waits for the screen's refresh (vsync); every GPU has it.
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto),
        view_formats: Vec::new(),
        color_space: wgpu::SurfaceColorSpace::Auto,
    };
    surface.configure(&g.device, &config);
    let mut d = drawing();
    d.next += 1;
    let n = d.next;
    d.screens.insert(n, Screen { surface, config });
    Ok(n)
}

/// The next picture of screen `n`, whose window is now `width` × `height`
/// pixels: its number, or why there is none now (try again next frame).
pub fn frame(n: i64, width: u32, height: u32) -> Result<i64, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let mut d = drawing();
    let screen = d.screens.get_mut(&n).ok_or("the screen is closed")?;
    if width == 0 || height == 0 {
        return Err("the window is minimized".into());
    }
    if (screen.config.width, screen.config.height) != (width, height) {
        screen.config.width = width;
        screen.config.height = height;
        screen.surface.configure(&g.device, &screen.config);
    }
    let texture = match screen.surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
        other => {
            // Lost or out of date: configured again, ready for the next try.
            screen.surface.configure(&g.device, &screen.config);
            return Err(format!("no picture this time: {other:?}"));
        }
    };
    let view = texture
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = g
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kavach frame"),
        });
    d.next += 1;
    let f = d.next;
    d.frames.insert(
        f,
        Frame {
            screen: n,
            texture,
            view,
            encoder,
        },
    );
    Ok(f)
}

/// Fills frame `f` with one color, each part from 0 to 1.
pub fn clear(f: i64, r: f64, g: f64, b: f64) {
    let mut d = drawing();
    let Some(frame) = d.frames.get_mut(&f) else {
        return;
    };
    let color = wgpu::Color { r, g, b, a: 1.0 };
    let pass = frame
        .encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kavach clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    drop(pass);
}

/// Sends what was drawn into frame `f` and puts it on the screen.
pub fn show(f: i64) {
    let Some(frame) = drawing().frames.remove(&f) else {
        return;
    };
    if let Some(g) = gpu() {
        g.queue.submit([frame.encoder.finish()]);
        g.queue.present(frame.texture);
    }
}

/// Lets frame `f` go: a frame not shown is thrown away.
pub fn drop_frame(f: i64) {
    drawing().frames.remove(&f);
}

/// Lets screen `n` go, with any frame of it not shown.
pub fn close(n: i64) {
    let mut d = drawing();
    d.frames.retain(|_, f| f.screen != n);
    d.screens.remove(&n);
}
