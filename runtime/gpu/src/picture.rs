// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Drawing into a picture off the screen (decision 0106): for tests now,
//! and later for programs that draw into a texture.

use crate::device::gpu;
use crate::render::{Draw, Target, draw_into, has_depth, ready_for};

/// Draws `draws`, in turn, into a new picture of `size` pixels, cleared
/// to black with every depth at 1, and gives its pixels: red, green, blue
/// and opacity, row by row from the top. Drawing into a texture, for
/// tests and, later, for programs (decision 0106).
pub fn picture(draws: Vec<Draw<'_>>, size: (u32, u32)) -> Result<Vec<[u8; 4]>, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let (w, h) = (size.0.max(1), size.1.max(1));
    let format = wgpu::TextureFormat::Rgba8Unorm;
    ready_for(format);
    let texture = g.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kavach picture"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = g
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kavach picture"),
        });
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("kavach picture"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    }));
    let depths = (draws.iter().any(|d| has_depth(d.pipeline)))
        .then(|| crate::texture::depth(w, h))
        .flatten();
    let mut fresh = true;
    for d in draws {
        let deep = has_depth(d.pipeline);
        let target = Target {
            view: &view,
            format,
            depth: depths.as_ref().map(|v| (v, fresh)),
        };
        draw_into(target, &mut encoder, d)?;
        fresh &= !deep;
    }
    // Rows are copied 256 bytes apart.
    let row = (4 * w).div_ceil(256) * 256;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kavach picture"),
        size: u64::from(row * h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &out,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    let index = g.queue.submit([encoder.finish()]);
    g.wait_mapped_for(&out, wgpu::MapMode::Read, Some(index))
        .map_err(|_| "the GPU stopped")?;
    let bytes = out
        .slice(..)
        .get_mapped_range()
        .map_err(|e| e.to_string())?
        .to_vec();
    out.unmap();
    let mut pixels = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let at = (y * row + 4 * x) as usize;
            pixels.push([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        }
    }
    Ok(pixels)
}
