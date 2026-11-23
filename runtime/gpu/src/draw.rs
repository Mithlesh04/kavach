// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Drawing into windows and canvases (spec 14.24, decision 0106), by
//! number, as the interpreter and native code call it. A place is what
//! frames draw into: a screen, a surface on a window made from the
//! instance the program's device came from and sized to the window at
//! each frame; or a canvas, a texture of its own (spec 14.24.1). Either
//! may draw with several samples a pixel, resolved into the picture. A
//! frame is the place's next picture: what is drawn into it is recorded,
//! and `show` sends it.

use crate::devices::gpu;
use crate::render::{Mask, State};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// What a place draws into: a window's surface, or a canvas's texture.
enum Kind {
    Surface {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
        /// The window's number (`kavach_runtime_window`).
        window: i64,
    },
    Canvas {
        texture: wgpu::Texture,
        view: Arc<wgpu::TextureView>,
    },
}

struct Place {
    kind: Kind,
    format: wgpu::TextureFormat,
    samples: u32,
    /// Its picture of several samples a pixel, and their size.
    multi: Option<(wgpu::TextureView, (u32, u32))>,
    /// Its depths and mask, made at the first draw needing them, and
    /// their size.
    depth: Option<(wgpu::TextureView, (u32, u32))>,
}

impl Place {
    fn size(&self) -> (u32, u32) {
        match &self.kind {
            Kind::Surface { config, .. } => (config.width, config.height),
            Kind::Canvas { texture, .. } => (texture.width(), texture.height()),
        }
    }
}

struct Frame {
    place: i64,
    /// The surface's picture, shown at `show`; none for a canvas.
    texture: Option<wgpu::SurfaceTexture>,
    view: wgpu::TextureView,
    encoder: wgpu::CommandEncoder,
    /// Whether its depths, and its mask, are still to be cleared.
    fresh: bool,
    fresh_mask: bool,
    /// How it draws now (spec 14.24.1).
    state: State,
    /// Its GPU time, when a trace is written (spec 14.12).
    timer: Option<crate::timers::Timer>,
}

#[derive(Default)]
struct Drawing {
    places: HashMap<i64, Place>,
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

/// The samples a pixel a place may have: 1 or 4, which every GPU has.
fn samples_ok(samples: u32) -> Result<u32, String> {
    match samples {
        1 | 4 => Ok(samples),
        _ => Err(format!(
            "a picture has 1 or 4 samples a pixel, not {samples}"
        )),
    }
}

fn add(place: Place) -> i64 {
    let (format, samples) = (place.format, place.samples);
    let mut d = drawing();
    d.next += 1;
    let n = d.next;
    d.places.insert(n, place);
    drop(d);
    // Compiled here, on the program's thread, not on the window's.
    crate::pipelines::ready_for(format, samples);
    n
}

/// A screen on window number `n` of `kavach_runtime_window`, now `size`
/// pixels, of `samples` a pixel: its number, or why it can't be drawn
/// into. The surface is made on the window's own thread, where every
/// system gives its handle; the device opens before, so the window's
/// events don't wait for it.
pub fn screen_on(n: i64, size: (u32, u32), samples: u32) -> Result<i64, String> {
    let samples = samples_ok(samples)?;
    if gpu().is_none() {
        return Err("this machine has no GPU".into());
    }
    let window = n;
    let place = kavach_runtime_window::with_window(n, move |w| surface(w, window, size))
        .unwrap_or_else(|| Err("the window is closed".into()))?;
    Ok(add(Place { samples, ..place }))
}

/// A place on `window`, number `number`, now `size` pixels; or why it
/// can't be drawn into.
fn surface(
    window: impl Into<wgpu::SurfaceTarget<'static>>,
    number: i64,
    size: (u32, u32),
) -> Result<Place, String> {
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
        width: size.0.max(1),
        height: size.1.max(1),
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
    Ok(Place {
        kind: Kind::Surface {
            surface,
            config,
            window: number,
        },
        format,
        samples: 1,
        multi: None,
        depth: None,
    })
}

/// A canvas `width` × `height` pixels of `samples` a pixel, cleared to
/// transparent black: its number, or why there is none (spec 14.24.1).
pub fn canvas(width: u32, height: u32, samples: u32) -> Result<i64, String> {
    let samples = samples_ok(samples)?;
    let most = crate::texture::MOST;
    if !(1..=most).contains(&width) || !(1..=most).contains(&height) {
        return Err(format!(
            "a canvas is 1 to {most} pixels wide and high, not {width} × {height}"
        ));
    }
    let g = gpu().ok_or("this machine has no GPU")?;
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach canvas"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
    Ok(add(Place {
        kind: Kind::Canvas { texture, view },
        format,
        samples,
        multi: None,
        depth: None,
    }))
}

/// Canvas `n`'s pixels as its last frame shown left them: four bytes a
/// pixel, in sRGB, row by row from the top.
pub fn canvas_pixels(n: i64) -> Result<Vec<u8>, String> {
    let texture = match drawing().places.get(&n).map(|p| &p.kind) {
        Some(Kind::Canvas { texture, .. }) => texture.clone(),
        _ => return Err("the canvas is gone".into()),
    };
    crate::picture::read_back(&texture, 4)
}

/// Canvas `n` as a texture for fragment fns to read: its number.
pub fn canvas_texture(n: i64, nearest: bool) -> Result<i64, String> {
    match drawing().places.get(&n).map(|p| &p.kind) {
        Some(Kind::Canvas { view, .. }) => {
            Ok(crate::texture::keep(Arc::clone(view), None, nearest, 4))
        }
        _ => Err("the canvas is gone".into()),
    }
}

/// A picture of `samples` a pixel, `size` pixels, in `format`.
fn multisampled(
    format: wgpu::TextureFormat,
    size: (u32, u32),
    samples: u32,
) -> Option<wgpu::TextureView> {
    let g = gpu()?;
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach samples"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

/// The next picture of place `n`, a screen whose window is now `width` ×
/// `height` pixels or a canvas: its number, or why there is none now
/// (try again next frame).
pub fn frame(n: i64, width: u32, height: u32) -> Result<i64, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let mut d = drawing();
    let place = d.places.get_mut(&n).ok_or("the screen is closed")?;
    let (texture, view) = match &mut place.kind {
        Kind::Canvas { view, .. } => (None, (**view).clone()),
        Kind::Surface {
            surface, config, ..
        } => {
            if width == 0 || height == 0 {
                return Err("the window is minimized".into());
            }
            if (config.width, config.height) != (width, height) {
                config.width = width;
                config.height = height;
                surface.configure(&g.device, config);
            }
            let texture = match surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(t)
                | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
                other => {
                    // Lost or out of date: configured again, for the next try.
                    surface.configure(&g.device, config);
                    return Err(format!("no picture this time: {other:?}"));
                }
            };
            let view = (texture.texture).create_view(&wgpu::TextureViewDescriptor::default());
            (Some(texture), view)
        }
    };
    let size = place.size();
    if place.samples > 1 && place.multi.as_ref().is_none_or(|(_, s)| *s != size) {
        let multi =
            multisampled(place.format, size, place.samples).ok_or("this machine has no GPU")?;
        place.multi = Some((multi, size));
    }
    let mut encoder = g
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kavach frame"),
        });
    let timer = crate::timers::start(&g.device, &mut encoder, "gpu frame");
    d.next += 1;
    let f = d.next;
    d.frames.insert(
        f,
        Frame {
            place: n,
            texture,
            view,
            encoder,
            fresh: true,
            fresh_mask: true,
            state: State::default(),
            timer,
        },
    );
    Ok(f)
}

/// Fills frame `f` with one color, each part from 0 to 1, with every
/// depth at 1 and an empty mask.
pub fn clear(f: i64, r: f64, g: f64, b: f64) {
    let mut d = drawing();
    let Drawing { places, frames, .. } = &mut *d;
    let Some(frame) = frames.get_mut(&f) else {
        return;
    };
    let Some(place) = places.get(&frame.place) else {
        return;
    };
    (frame.fresh, frame.fresh_mask) = (true, true);
    let color = wgpu::Color { r, g, b, a: 1.0 };
    let (view, resolve) = match &place.multi {
        Some((multi, _)) => (multi, Some(&frame.view)),
        None => (&frame.view, None),
    };
    let pass = frame
        .encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("kavach clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    drop(pass);
}

/// Changes how frame `f` draws from now on (spec 14.24.1): `what` 0 its
/// clip (`values` `x`, `y`, width and height, or none for no clip), 1
/// premultiplied blending (`values[0]` 1 or 0), 2 its use of the mask
/// (`values[0]`, `Mask`'s number), 3 its mask emptied.
pub fn set_state(f: i64, what: i64, values: &[i64]) {
    let mut d = drawing();
    let Some(frame) = d.frames.get_mut(&f) else {
        return;
    };
    let first = values.first().copied().unwrap_or(0);
    let fit = |v: i64| u32::try_from(v.max(0)).unwrap_or(u32::MAX);
    match what {
        0 => {
            frame.state.clip = match values {
                [x, y, w, h] => Some([fit(*x), fit(*y), fit(*w), fit(*h)]),
                _ => None,
            }
        }
        1 => frame.state.premultiplied = first != 0,
        2 => frame.state.mask = Mask::of(first),
        _ => frame.fresh_mask = true,
    }
}

/// Makes `draw` into frame `f` (spec 15.3), with the place's depths and
/// mask when it needs them.
pub fn draw(f: i64, draw: crate::render::Draw<'_>) -> Result<(), String> {
    let mut d = drawing();
    let Drawing { places, frames, .. } = &mut *d;
    let frame = frames.get_mut(&f).ok_or("the frame is gone")?;
    let place = places.get_mut(&frame.place).ok_or("the screen is closed")?;
    if let Kind::Canvas { view, .. } = &place.kind
        && draw
            .textures
            .iter()
            .any(|t| crate::texture::is_view(*t, view))
    {
        return Err("a canvas can't be read by a draw into itself".into());
    }
    let size = place.size();
    if crate::render::needs_depth(draw.pipeline, &frame.state)
        && place.depth.as_ref().is_none_or(|(_, s)| *s != size)
    {
        let view = crate::texture::depth(size.0, size.1, place.samples)
            .ok_or("this machine has no GPU")?;
        place.depth = Some((view, size));
        (frame.fresh, frame.fresh_mask) = (true, true);
    }
    let (view, resolve) = match &place.multi {
        Some((multi, _)) => (multi, Some(&frame.view)),
        None => (&frame.view, None),
    };
    let target = crate::render::Target {
        view,
        resolve,
        format: place.format,
        samples: place.samples,
        size,
        depth: (place.depth.as_ref()).map(|(v, _)| (v, frame.fresh, frame.fresh_mask)),
        state: frame.state,
    };
    let deep = crate::render::has_depth(draw.pipeline);
    crate::render::draw_into(target, &mut frame.encoder, draw)?;
    frame.fresh &= !deep;
    frame.fresh_mask &= frame.state.mask == Mask::Off;
    Ok(())
}

/// Sends what was drawn into frame `f`, and puts a screen's on the
/// screen, which answers a `Redraw` its window's event loop waits on
/// (spec 14.22).
pub fn show(f: i64) {
    let mut d = drawing();
    let Some(mut frame) = d.frames.remove(&f) else {
        return;
    };
    let window = match d.places.get(&frame.place).map(|p| &p.kind) {
        Some(Kind::Surface { window, .. }) => Some(*window),
        _ => None,
    };
    drop(d);
    if let Some(g) = gpu() {
        if let Some(t) = &frame.timer {
            crate::timers::stop(&mut frame.encoder, t);
        }
        g.queue.submit([frame.encoder.finish()]);
        crate::timers::sent(&g.device, &g.queue, frame.timer);
        if let Some(texture) = frame.texture {
            g.queue.present(texture);
        }
    }
    if let Some(w) = window {
        kavach_runtime_window::frame_shown(w);
    }
}

/// Lets frame `f` go: a frame not shown is thrown away.
pub fn drop_frame(f: i64) {
    drawing().frames.remove(&f);
}

/// Lets place `n` go, with any frame of it not shown.
pub fn close(n: i64) {
    let mut d = drawing();
    d.frames.retain(|_, f| f.place != n);
    d.places.remove(&n);
}
