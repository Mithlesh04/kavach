// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Pipelines and draws (spec 15.3, decision 0106). A pipeline is its WGSL
//! module, written by the compiler (`compiler/backend/wgsl/src/render.rs`),
//! whose first line gives the buffers' layout: `// draw f f | f f f | 1 |
//! 0 | 0`, the vertex's fields, the instance's, the shared value's words,
//! whether it has depth (1) and how many textures its fragment fn takes. It
//! is compiled ahead, never in the middle of a frame (decision 0106): for
//! every screen's color format when the screen opens, and when it is made
//! while a screen is open.

use crate::device::gpu;
pub use crate::picture::picture;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use wgpu::util::DeviceExt;

/// What the first line of a pipeline's module says.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub vertex: Vec<wgpu::VertexFormat>,
    pub instance: Vec<wgpu::VertexFormat>,
    pub shared: usize,
    /// Whether a pixel is drawn only where it is nearer (spec 15.3).
    pub depth: bool,
    /// The textures its fragment fn takes.
    pub textures: usize,
}

/// The layout the first line of `wgsl` gives.
pub fn layout(wgsl: &str) -> Option<Layout> {
    let line = wgsl.lines().next()?.strip_prefix("// draw ")?;
    let parts: Vec<&str> = line.split('|').map(str::trim).collect();
    let (vertex, instance, shared, depth, textures) = match parts[..] {
        [v, i, s] => (v, i, s, "0", "0"),
        [v, i, s, d, t] => (v, i, s, d, t),
        _ => return None,
    };
    let formats = |s: &str| {
        s.split_whitespace()
            .map(|f| match f {
                "f" => Some(wgpu::VertexFormat::Float32),
                "i" => Some(wgpu::VertexFormat::Sint32),
                "u" => Some(wgpu::VertexFormat::Uint32),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
    };
    Some(Layout {
        vertex: formats(vertex)?,
        instance: formats(instance)?,
        shared: shared.parse().ok()?,
        depth: depth == "1",
        textures: textures.parse().ok()?,
    })
}

struct Made {
    pipeline: wgpu::RenderPipeline,
    shared: Option<wgpu::BindGroupLayout>,
    textures: Option<wgpu::BindGroupLayout>,
}

struct Pipeline {
    wgsl: String,
    layout: Layout,
    /// Compiled for each color format it has drawn into.
    made: HashMap<wgpu::TextureFormat, Arc<Made>>,
}

#[derive(Default)]
struct Pipelines {
    /// The color formats of the screens opened so far.
    formats: Vec<wgpu::TextureFormat>,
    all: HashMap<i64, Pipeline>,
    by_text: HashMap<String, i64>,
}

fn pipelines() -> MutexGuard<'static, Pipelines> {
    static PIPELINES: OnceLock<Mutex<Pipelines>> = OnceLock::new();
    PIPELINES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// The number of the pipeline of module `wgsl`: the same for the same
/// text. It is compiled now for each screen open.
pub fn pipeline(wgsl: &str) -> Result<i64, String> {
    let layout = layout(wgsl).ok_or("this pipeline's shaders couldn't be written for the GPU")?;
    let mut p = pipelines();
    if let Some(&n) = p.by_text.get(wgsl) {
        return Ok(n);
    }
    let n = p.all.len() as i64 + 1;
    p.all.insert(
        n,
        Pipeline {
            wgsl: wgsl.to_string(),
            layout,
            made: HashMap::new(),
        },
    );
    p.by_text.insert(wgsl.to_string(), n);
    let formats = p.formats.clone();
    drop(p);
    for format in formats {
        made(n, format);
    }
    Ok(n)
}

/// Compiles every pipeline made so far for a screen of `format`, and
/// those made later as they come, so frames never wait for a compile.
pub(crate) fn ready_for(format: wgpu::TextureFormat) {
    let mut p = pipelines();
    if p.formats.contains(&format) {
        return;
    }
    p.formats.push(format);
    let all: Vec<i64> = p.all.keys().copied().collect();
    drop(p);
    for n in all {
        made(n, format);
    }
}

/// Whether pipeline `n` is compiled for some screen or picture yet.
pub fn is_made(n: i64) -> bool {
    pipelines().all.get(&n).is_some_and(|p| !p.made.is_empty())
}

/// Pipeline `n` compiled to draw into `format`, and its layout.
fn made(n: i64, format: wgpu::TextureFormat) -> Option<(Arc<Made>, Layout)> {
    let mut p = pipelines();
    let pipe = p.all.get_mut(&n)?;
    if let Some(m) = pipe.made.get(&format) {
        return Some((m.clone(), pipe.layout.clone()));
    }
    let m = Arc::new(compile(&pipe.wgsl, &pipe.layout, format)?);
    pipe.made.insert(format, m.clone());
    Some((m, pipe.layout.clone()))
}

fn attributes(formats: &[wgpu::VertexFormat], first: u32) -> Vec<wgpu::VertexAttribute> {
    (formats.iter().enumerate())
        .map(|(k, f)| wgpu::VertexAttribute {
            format: *f,
            offset: 4 * k as u64,
            shader_location: first + k as u32,
        })
        .collect()
}

fn compile(wgsl: &str, layout: &Layout, format: wgpu::TextureFormat) -> Option<Made> {
    let g = gpu()?;
    let module = g.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("kavach pipeline"),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });
    let shared = (layout.shared > 0).then(|| {
        g.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("kavach shared"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
    });
    let textures = (layout.textures > 0)
        .then(|| crate::texture::layout(layout.textures))
        .flatten();
    let mut groups: Vec<Option<&wgpu::BindGroupLayout>> = vec![shared.as_ref()];
    if let Some(t) = &textures {
        groups.push(Some(t));
    }
    while groups.last() == Some(&None) {
        groups.pop();
    }
    let pipeline_layout = g
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kavach pipeline"),
            bind_group_layouts: &groups,
            immediate_size: 0,
        });
    let vertex = attributes(&layout.vertex, 0);
    let instance = attributes(&layout.instance, layout.vertex.len() as u32);
    let mut buffers = vec![Some(wgpu::VertexBufferLayout {
        array_stride: 4 * layout.vertex.len() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &vertex,
    })];
    if !instance.is_empty() {
        buffers.push(Some(wgpu::VertexBufferLayout {
            array_stride: 4 * layout.instance.len() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &instance,
        }));
    }
    let pipeline = g
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kavach pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState::default(),
            // Nearer pixels only, each keeping its depth (spec 15.3).
            depth_stencil: layout.depth.then(|| wgpu::DepthStencilState {
                format: crate::texture::DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                // Opacity blends a pixel with what is drawn under it.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
    Some(Made {
        pipeline,
        shared,
        textures,
    })
}

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

/// Where a draw goes: the picture, its color format, and its depths, with
/// whether they are still to be set to 1, the farthest.
pub(crate) struct Target<'a> {
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub depth: Option<(&'a wgpu::TextureView, bool)>,
}

/// Whether pipeline `n` draws with depth, so its target needs depths.
pub fn has_depth(n: i64) -> bool {
    pipelines().all.get(&n).is_some_and(|p| p.layout.depth)
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
    let (m, layout) = made(draw.pipeline, target.format).ok_or("this pipeline couldn't be made")?;
    let textures = match &m.textures {
        Some(l) => Some(crate::texture::group(l, layout.textures, draw.textures)?),
        None => None,
    };
    let depth = match (layout.depth, target.depth) {
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
    // it is. Either way: the buffer, and how many items it holds.
    fn source<'b>(
        items: &Items<'b>,
        width: usize,
        make: impl Fn(&[u32]) -> wgpu::Buffer,
    ) -> (std::borrow::Cow<'b, wgpu::Buffer>, u32) {
        let width = width.max(1);
        match *items {
            Items::Words(w) => (std::borrow::Cow::Owned(make(w)), (w.len() / width) as u32),
            Items::Kept(r) => {
                let (b, words) = r.parts();
                (std::borrow::Cow::Borrowed(b), (words / width) as u32)
            }
        }
    }
    let vertex = |w: &[u32]| buffer(w, wgpu::BufferUsages::VERTEX);
    let (vb, count) = source(&vertices, layout.vertex.len(), vertex);
    let ib =
        (!layout.instance.is_empty()).then(|| source(&instances, layout.instance.len(), vertex));
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
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: depth.map(|(view, fresh)| {
            wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: if fresh {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }
        }),
        ..Default::default()
    });
    pass.set_pipeline(&m.pipeline);
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
