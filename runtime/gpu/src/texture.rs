// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Textures (spec 15.3): pictures a `gpu fragment fn` reads with
//! `sample`, kept on the GPU by number. A pipeline whose fragment fn takes
//! textures has a second bind group: one sampler, blending between the
//! nearest pixels and clamping at the edges, then each texture in turn.

use crate::device::gpu;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// The largest width or height a texture may have (spec 14.24).
pub const MOST: u32 = 8192;

#[derive(Default)]
struct Textures {
    all: HashMap<i64, Arc<wgpu::TextureView>>,
    next: i64,
}

fn textures() -> MutexGuard<'static, Textures> {
    static TEXTURES: OnceLock<Mutex<Textures>> = OnceLock::new();
    TEXTURES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// A texture `width` × `height` pixels of `pixels`, four bytes each (red,
/// green, blue and opacity, in sRGB), row by row from the top: its
/// number, or why there is none.
pub fn texture(width: u32, height: u32, pixels: &[u8]) -> Result<i64, String> {
    if !(1..=MOST).contains(&width) || !(1..=MOST).contains(&height) {
        return Err(format!(
            "a texture is 1 to {MOST} pixels wide and high, not {width} × {height}"
        ));
    }
    if pixels.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err(format!(
            "a texture {width} × {height} takes {} bytes of pixels, and {} were given",
            u64::from(width) * u64::from(height) * 4,
            pixels.len()
        ));
    }
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
        // Sampled as linear colors, which the screen's sRGB turns back.
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    g.queue.write_texture(
        texture.as_image_copy(),
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut t = textures();
    t.next += 1;
    let n = t.next;
    t.all.insert(n, Arc::new(view));
    Ok(n)
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
    let mut all = textures();
    all.next += 1;
    let n = all.next;
    all.all.insert(n, Arc::new(view));
    Ok((n, t.width, t.height))
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

/// The one sampler: blending between the nearest pixels, the edge's
/// pixels past an edge (spec 15.3).
fn sampler() -> Option<&'static wgpu::Sampler> {
    static SAMPLER: OnceLock<Option<wgpu::Sampler>> = OnceLock::new();
    SAMPLER
        .get_or_init(|| {
            let g = gpu()?;
            Some(g.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("kavach sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }))
        })
        .as_ref()
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
    let views: Vec<Arc<wgpu::TextureView>> = {
        let t = textures();
        (ids.iter())
            .map(|n| {
                t.all
                    .get(n)
                    .cloned()
                    .ok_or("a texture of this draw is gone")
            })
            .collect::<Result<_, _>>()?
    };
    let sampler = sampler().ok_or("this machine has no GPU")?;
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

/// A depth buffer `width` × `height` pixels, for pipelines with depth.
pub(crate) fn depth(width: u32, height: u32) -> Option<wgpu::TextureView> {
    let g = gpu()?;
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach depth"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    Some(texture.create_view(&wgpu::TextureViewDescriptor::default()))
}

/// The format of every depth buffer.
pub(crate) const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
