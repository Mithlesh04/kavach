// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Draws (spec 15.3, decision 0106): a pipeline's triangles from 32-bit
//! words, or from a list a GPU `map` left in GPU memory, into a frame's
//! picture, with its depths and mask, inside its clip, blended as its
//! frame says (spec 14.24.1). The pipelines themselves are in
//! `pipelines.rs`.

use crate::devices::gpu;
pub use crate::picture::{picture, picture_with};
use crate::pipelines::{Form, made};
pub use crate::pipelines::{Layout, Mask, has_depth, is_made, layout, pipeline};
use wgpu::util::DeviceExt;

/// A draw's vertices or instances: 32-bit words, the fields in turn, or a
/// list a GPU `map` left in GPU memory, drawn from there (decision 0106).
pub enum Items<'a> {
    Words(&'a [u32]),
    Kept(&'a crate::Resident),
}

/// One draw: through pipeline `pipeline`, its vertices, instances and
/// shared value (words), the numbers of the textures its fragment fn
/// reads (spec 15.3), and its indices (spec 14.24).
pub struct Draw<'a> {
    pub pipeline: i64,
    pub parts: (Items<'a>, Items<'a>, &'a [u32]),
    pub textures: &'a [i64],
    /// The vertices of each triangle by their place, when given.
    pub indices: Option<&'a [u32]>,
}

/// How a frame draws now (spec 14.24.1): only inside its clip, `x`, `y`,
/// width and height in pixels from the top left; blending premultiplied
/// colors or not; and its use of the mask.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub clip: Option<[u32; 4]>,
    pub premultiplied: bool,
    pub mask: Mask,
}

/// Where a draw goes: the picture (with samples a pixel, the one they
/// are resolved into), its color format and size, its depths and mask
/// (with whether each is still to be cleared), and how it draws now.
pub(crate) struct Target<'a> {
    pub view: &'a wgpu::TextureView,
    pub resolve: Option<&'a wgpu::TextureView>,
    pub format: wgpu::TextureFormat,
    pub samples: u32,
    pub size: (u32, u32),
    pub depth: Option<(&'a wgpu::TextureView, bool, bool)>,
    pub state: State,
}

/// Whether `draw` through pipeline `n` with `state` needs depths or a
/// mask in its target.
pub fn needs_depth(n: i64, state: &State) -> bool {
    has_depth(n) || state.mask != Mask::Off
}

/// The part of a picture of `size` a clip leaves: `None` when nothing.
fn clipped(clip: Option<[u32; 4]>, size: (u32, u32)) -> Option<[u32; 4]> {
    let [x, y, w, h] = clip.unwrap_or([0, 0, size.0, size.1]);
    let (x0, y0) = (x.min(size.0), y.min(size.1));
    let (x1, y1) = (
        x.saturating_add(w).min(size.0),
        y.saturating_add(h).min(size.1),
    );
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

/// Draws `vertices` through the pipeline into the target, recorded on
/// `encoder`: once for each of `instances` when the pipeline takes an
/// instance, with `shared` given to every corner. What is there already
/// stays under it, or, with depth, over it where it is nearer.
pub(crate) fn draw_into(
    target: Target<'_>,
    encoder: &mut wgpu::CommandEncoder,
    draw: Draw<'_>,
) -> Result<(), String> {
    let (vertices, instances, shared) = draw.parts;
    let g = gpu().ok_or("this machine has no GPU")?;
    let state = target.state;
    let form = Form {
        format: target.format,
        samples: target.samples,
        premultiplied: state.premultiplied,
        mask: state.mask,
    };
    let (m, layout) = made(draw.pipeline, form).ok_or("this pipeline couldn't be made")?;
    let textures = match &m.textures {
        Some(l) => Some(crate::texture::group(l, layout.textures, draw.textures)?),
        None => None,
    };
    let depth = match (layout.depth || state.mask != Mask::Off, target.depth) {
        (false, _) => None,
        (true, Some(d)) => Some(d),
        (true, None) => return Err("this picture has no depths".into()),
    };
    let buffer = |words: &[u32], usage| {
        g.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("kavach draw"),
                contents: &bytes_of(if words.is_empty() { &[0] } else { words }),
                usage,
            })
    };
    // Words go over in a buffer of their own; a kept list is drawn where
    // it is, or copied over from another device's memory (spec 15.6).
    // Either way: the buffer, and how many items it holds.
    fn source<'b>(
        items: &Items<'b>,
        width: usize,
        here: usize,
        make: impl Fn(&[u32]) -> wgpu::Buffer,
    ) -> (std::borrow::Cow<'b, wgpu::Buffer>, u32) {
        let width = width.max(1);
        match *items {
            Items::Words(w) => (std::borrow::Cow::Owned(make(w)), (w.len() / width) as u32),
            Items::Kept(r) if r.device() == here => {
                let (b, words) = r.parts();
                (std::borrow::Cow::Borrowed(b), (words / width) as u32)
            }
            Items::Kept(r) => {
                let mut bytes = vec![0u8; r.bytes()];
                let _ = crate::read(r, &mut bytes);
                let w: Vec<u32> = (bytes.as_chunks::<4>().0.iter())
                    .map(|b| u32::from_le_bytes(*b))
                    .collect();
                (std::borrow::Cow::Owned(make(&w)), (w.len() / width) as u32)
            }
        }
    }
    let vertex = |w: &[u32]| buffer(w, wgpu::BufferUsages::VERTEX);
    let (vb, count) = source(&vertices, layout.vertex.len(), g.index, vertex);
    let ib = (!layout.instance.is_empty())
        .then(|| source(&instances, layout.instance.len(), g.index, vertex));
    let times = ib.as_ref().map_or(1, |(_, n)| *n);
    // Every index names a vertex there is (spec 14.24).
    let indices = match draw.indices {
        Some(ix) => {
            if let Some(past) = ix.iter().find(|k| **k >= count) {
                return Err(format!(
                    "index {past} is past the last of the {count} vertices"
                ));
            }
            Some((buffer(ix, wgpu::BufferUsages::INDEX), ix.len() as u32))
        }
        None => None,
    };
    // A clip that leaves nothing draws nothing.
    let Some([cx, cy, cw, ch]) = clipped(state.clip, target.size) else {
        return Ok(());
    };
    let group = m.shared.as_ref().map(|l| {
        let sb = buffer(shared, wgpu::BufferUsages::STORAGE);
        g.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kavach shared"),
            layout: l,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sb.as_entire_binding(),
            }],
        })
    });
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("kavach draw"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target.view,
            depth_slice: None,
            resolve_target: target.resolve,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: depth.map(|(view, fresh, fresh_mask)| {
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: layout.depth.then_some(wgpu::Operations {
                    load: if fresh {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: (state.mask != Mask::Off).then_some(wgpu::Operations {
                    load: if fresh_mask {
                        wgpu::LoadOp::Clear(0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
            }
        }),
        ..Default::default()
    });
    pass.set_pipeline(&m.pipeline);
    pass.set_scissor_rect(cx, cy, cw, ch);
    pass.set_stencil_reference(1);
    if let Some(group) = &group {
        pass.set_bind_group(0, group, &[]);
    }
    if let Some(group) = &textures {
        pass.set_bind_group(1, group, &[]);
    }
    pass.set_vertex_buffer(0, vb.slice(..));
    if let Some((ib, _)) = &ib {
        pass.set_vertex_buffer(1, ib.slice(..));
    }
    match &indices {
        Some((buffer, n)) => {
            pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..*n, 0, 0..times);
        }
        None => pass.draw(0..count, 0..times),
    }
    Ok(())
}

/// Words as bytes, little-endian as GPUs read them.
fn bytes_of(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::clipped;

    #[test]
    fn a_clip_is_cut_to_the_picture() {
        assert_eq!(clipped(None, (8, 6)), Some([0, 0, 8, 6]));
        assert_eq!(clipped(Some([2, 1, 3, 2]), (8, 6)), Some([2, 1, 3, 2]));
        assert_eq!(clipped(Some([6, 4, 10, 10]), (8, 6)), Some([6, 4, 2, 2]));
        assert_eq!(clipped(Some([9, 0, 3, 3]), (8, 6)), None, "past the edge");
        assert_eq!(clipped(Some([1, 1, 0, 4]), (8, 6)), None, "no width");
        assert_eq!(clipped(Some([u32::MAX, 0, u32::MAX, 1]), (8, 6)), None);
    }
}
