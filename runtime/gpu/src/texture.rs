// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Textures (spec 15.3): pictures a `gpu fragment fn` reads with
//! `sample`, kept on the GPU by number. A pipeline whose fragment fn takes
//! textures has a second bind group: one sampler, blending between the
//! nearest pixels or taking the nearest one, and clamping at the edges,
//! then each texture in turn. A texture has four channels (sRGB colours
//! and opacity) or one (spec 14.24.1), and part of it can be changed.

use crate::devices::gpu;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// The largest width or height a texture may have (spec 14.24).
pub const MOST: u32 = 8192;

/// A texture kept by number: what draws read, and what it is.
pub(crate) struct Kept {
    pub(crate) view: Arc<wgpu::TextureView>,
    /// Kept for changing part of it; `None` for compressed blocks.
    texture: Option<wgpu::Texture>,
    /// Sampled to the nearest pixel, not blended (spec 14.24.1).
    nearest: bool,
    /// One channel, or four.
    channels: u32,
}

#[derive(Default)]
struct Textures {
    all: HashMap<i64, Kept>,
    next: i64,
}

/// Keeps a texture: its number.
pub(crate) fn keep(
    view: Arc<wgpu::TextureView>,
    texture: Option<wgpu::Texture>,
    nearest: bool,
    channels: u32,
) -> i64 {
    let mut t = textures();
    t.next += 1;
    let n = t.next;
    t.all.insert(
        n,
        Kept {
            view,
            texture,
            nearest,
            channels,
        },
    );
    n
}

fn textures() -> MutexGuard<'static, Textures> {
    static TEXTURES: OnceLock<Mutex<Textures>> = OnceLock::new();
    TEXTURES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Why `pixels` can't be a `width` × `height` picture of `channels`
/// bytes a pixel, if they can't.
fn check(width: u32, height: u32, channels: u32, pixels: &[u8]) -> Result<(), String> {
    if !(1..=MOST).contains(&width) || !(1..=MOST).contains(&height) {
        return Err(format!(
            "a texture is 1 to {MOST} pixels wide and high, not {width} × {height}"
        ));
    }
    let want = u64::from(width) * u64::from(height) * u64::from(channels);
    if pixels.len() as u64 != want {
        return Err(format!(
            "{width} × {height} pixels take {want} bytes, and {} were given",
            pixels.len()
        ));
    }
    Ok(())
}

/// A texture `width` × `height` pixels of `pixels`, four bytes each (red,
/// green, blue and opacity, in sRGB), row by row from the top: its
/// number, or why there is none.
pub fn texture(width: u32, height: u32, pixels: &[u8]) -> Result<i64, String> {
    texture_of(width, height, pixels, 4, false)
}

/// A texture of `channels` bytes a pixel (4, as `texture` takes, or 1,
/// a value from 0 to 1 each, not a colour), sampled to the `nearest`
/// pixel or blended: its number, or why there is none (spec 14.24.1).
pub fn texture_of(
    width: u32,
    height: u32,
    pixels: &[u8],
    channels: u32,
    nearest: bool,
) -> Result<i64, String> {
    check(width, height, channels, pixels)?;
    let g = gpu().ok_or("this machine has no GPU")?;
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach texture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Sampled as linear colors, which the screen's sRGB turns back;
        // one channel is a value, as it is.
        format: if channels == 1 {
            wgpu::TextureFormat::R8Unorm
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        },
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    g.queue.write_texture(
        texture.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(channels * width),
            rows_per_image: Some(height),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok(keep(Arc::new(view), Some(texture), nearest, channels))
}

/// Changes the part of texture `n` from pixel `(x, y)`, `width` ×
/// `height` pixels, to `pixels`, row by row from the top; or why not.
/// Draws already sent keep what they read (spec 14.24.1).
pub fn update(n: i64, at: (u32, u32), size: (u32, u32), pixels: &[u8]) -> Result<(), String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let t = textures();
    let kept = t.all.get(&n).ok_or("the texture is gone")?;
    let texture = (kept.texture.as_ref()).ok_or("a compressed texture can't be changed")?;
    let ((x, y), (w, h)) = (at, size);
    check(w, h, kept.channels, pixels)?;
    if u64::from(x) + u64::from(w) > u64::from(texture.width())
        || u64::from(y) + u64::from(h) > u64::from(texture.height())
    {
        return Err(format!(
            "{w} × {h} pixels at ({x}, {y}) go past the texture's {} × {}",
            texture.width(),
            texture.height()
        ));
    }
    g.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d { x, y, z: 0 },
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(kept.channels * w),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    Ok(())
}

/// A texture of the compressed blocks in KTX2 file `bytes` (`ktx2.rs`),
/// uploaded as they are with their mip levels: its number, width and
/// height; or why not, as `GpuError`'s variant and reason: `BadTexture`
/// for a file that can't be read, `Unavailable` for a GPU that can't.
pub fn compressed(bytes: &[u8]) -> Result<(i64, u32, u32), (&'static str, String)> {
    let t = crate::ktx2::read(bytes).map_err(|why| ("BadTexture", why))?;
    let g = gpu().ok_or(("Unavailable", "this machine has no GPU".to_string()))?;
    if !g.device.features().contains(t.format.required_features()) {
        return Err((
            "Unavailable",
            format!(
                "this GPU can't read {:?} textures: give it BC on a desktop, ASTC on a phone",
                t.format
            ),
        ));
    }
    let size = wgpu::Extent3d {
        width: t.width,
        height: t.height,
        depth_or_array_layers: 1,
    };
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach compressed texture"),
        size,
        mip_level_count: t.levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: t.format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let block = t.format.block_copy_size(None).unwrap_or(16);
    for (k, level) in t.levels.iter().enumerate() {
        let blocks = |v: u32| (v >> k).max(1).div_ceil(4);
        let (bx, by) = (blocks(t.width), blocks(t.height));
        g.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: k as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            level,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bx * block),
                rows_per_image: Some(by),
            },
            wgpu::Extent3d {
                width: bx * 4,
                height: by * 4,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Ok((keep(Arc::new(view), None, false, 4), t.width, t.height))
}

/// Whether texture `n` is `view`: a canvas read while it is drawn into.
pub(crate) fn is_view(n: i64, view: &Arc<wgpu::TextureView>) -> bool {
    textures()
        .all
        .get(&n)
        .is_some_and(|k| Arc::ptr_eq(&k.view, view))
}

/// Lets texture `n` go. A draw already recorded keeps it until it is sent.
pub fn drop_texture(n: i64) {
    textures().all.remove(&n);
}

/// The bind group layout of `count` textures and their sampler.
pub(crate) fn layout(count: usize) -> Option<wgpu::BindGroupLayout> {
    let g = gpu()?;
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }];
    for k in 0..count {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 1 + k as u32,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
    }
    Some(
        g.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("kavach textures"),
                entries: &entries,
            }),
    )
}

/// The two samplers: blending between the nearest pixels, or taking the
/// nearest one; the edge's pixels past an edge (spec 15.3).
fn sampler(nearest: bool) -> Option<&'static wgpu::Sampler> {
    static SAMPLERS: OnceLock<Option<[wgpu::Sampler; 2]>> = OnceLock::new();
    let both = SAMPLERS.get_or_init(|| {
        let g = gpu()?;
        let make = |filter| {
            g.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("kavach sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        Some([
            make(wgpu::FilterMode::Linear),
            make(wgpu::FilterMode::Nearest),
        ])
    });
    both.as_ref().map(|b| &b[usize::from(nearest)])
}

/// The bind group giving textures `ids`, in order, to a draw through a
/// pipeline of `layout`, which takes `count` of them.
pub(crate) fn group(
    layout: &wgpu::BindGroupLayout,
    count: usize,
    ids: &[i64],
) -> Result<wgpu::BindGroup, String> {
    if ids.len() != count {
        return Err(format!(
            "this pipeline's fragment fn takes {count} texture(s), and the draw gives {}",
            ids.len()
        ));
    }
    let g = gpu().ok_or("this machine has no GPU")?;
    let (views, nearest): (Vec<Arc<wgpu::TextureView>>, Vec<bool>) = {
        let t = textures();
        let kept = (ids.iter())
            .map(|n| t.all.get(n).ok_or("a texture of this draw is gone"))
            .collect::<Result<Vec<_>, _>>()?;
        kept.iter()
            .map(|k| (Arc::clone(&k.view), k.nearest))
            .unzip()
    };
    if nearest.iter().any(|n| *n != nearest[0]) {
        return Err("a draw's textures are all sampled to the nearest pixel, or none is".into());
    }
    let sampler = sampler(nearest[0]).ok_or("this machine has no GPU")?;
    let mut entries = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::Sampler(sampler),
    }];
    for (k, v) in views.iter().enumerate() {
        entries.push(wgpu::BindGroupEntry {
            binding: 1 + k as u32,
            resource: wgpu::BindingResource::TextureView(v),
        });
    }
    Ok(g.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("kavach textures"),
        layout,
        entries: &entries,
    }))
}

/// A depth and mask buffer `width` × `height` pixels, of `samples` a
/// pixel, for pipelines with depth or a mask (spec 14.24.1).
pub(crate) fn depth(width: u32, height: u32, samples: u32) -> Option<wgpu::TextureView> {
    let g = gpu()?;
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

/// The format of every depth buffer: depths, and the mask's mark.
pub(crate) const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
