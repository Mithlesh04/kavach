// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Drawing into a picture off the screen (decision 0106), for tests, and
//! reading a texture's pixels back, as a canvas's are (spec 14.24.1).

use crate::devices::gpu;
use crate::render::{Draw, State, Target, draw_into, needs_depth};

/// Draws `draws`, in turn, each with its state, into a new picture of
/// `size` pixels, cleared to black with every depth at 1 and an empty
/// mask, and gives its pixels: red, green, blue and opacity, row by row
/// from the top. For tests (decision 0106).
pub fn picture(draws: Vec<Draw<'_>>, size: (u32, u32)) -> Result<Vec<[u8; 4]>, String> {
    let with = draws.into_iter().map(|d| (d, State::default())).collect();
    picture_with(with, size)
}

/// `picture`, each draw with a state of its own (spec 14.24.1).
pub fn picture_with(
    draws: Vec<(Draw<'_>, State)>,
    size: (u32, u32),
) -> Result<Vec<[u8; 4]>, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let (w, h) = (size.0.max(1), size.1.max(1));
    let format = wgpu::TextureFormat::Rgba8Unorm;
    crate::pipelines::ready_for(format, 1);
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
    let depths = (draws.iter().any(|(d, s)| needs_depth(d.pipeline, s)))
        .then(|| crate::texture::depth(w, h, 1))
        .flatten();
    let (mut fresh, mut fresh_mask) = (true, true);
    for (d, state) in draws {
        let deep = crate::render::has_depth(d.pipeline);
        let target = Target {
            view: &view,
            resolve: None,
            format,
            samples: 1,
            size: (w, h),
            depth: depths.as_ref().map(|v| (v, fresh, fresh_mask)),
            state,
        };
        draw_into(target, &mut encoder, d)?;
        fresh &= !deep;
        fresh_mask &= state.mask == crate::render::Mask::Off;
    }
    g.queue.submit([encoder.finish()]);
    let bytes = read_back(&texture, 4)?;
    Ok(bytes.as_chunks::<4>().0.to_vec())
}

/// The pixels of `texture`, `channels` bytes each, row by row from the
/// top, once the GPU has done what it was sent.
pub(crate) fn read_back(texture: &wgpu::Texture, channels: u32) -> Result<Vec<u8>, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let (w, h) = (texture.width(), texture.height());
    // Rows are copied 256 bytes apart.
    let row = (channels * w).div_ceil(256) * 256;
    let out = g.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("kavach read back"),
        size: u64::from(row * h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = g
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("kavach read back"),
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
    let line = (channels * w) as usize;
    let mut pixels = Vec::with_capacity(line * h as usize);
    for y in 0..h as usize {
        let at = y * row as usize;
        pixels.extend_from_slice(&bytes[at..at + line]);
    }
    Ok(pixels)
}
