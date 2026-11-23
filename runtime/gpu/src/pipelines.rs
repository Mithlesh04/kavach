// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Pipelines (spec 15.3, decision 0106). A pipeline is its WGSL module,
//! written by the compiler (`compiler/backend/wgsl/src/render.rs`), whose
//! first line gives the buffers' layout: `// draw f f | f f f | 1 | 0 |
//! 0`, the vertex's fields, the instance's, the shared value's words,
//! whether it has depth (1) and how many textures its fragment fn takes.
//!
//! It is compiled in a form for each place it draws into: the color
//! format and samples a pixel, its blending and its use of the mask
//! (spec 14.24.1). The plain form is compiled ahead, never in the middle
//! of a frame: for every screen's format when the screen opens, and when
//! it is made while a screen is open. Another form is compiled at its
//! first draw.

use crate::devices::gpu;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

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

/// How a draw uses the frame's mask (spec 14.24.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Mask {
    /// Not at all.
    #[default]
    Off,
    /// It marks the pixels it covers, and colours none.
    Write,
    /// It draws only inside the marked pixels.
    Inside,
    /// It draws only outside them.
    Outside,
}

impl Mask {
    /// The mask of number `n`, as `std.gpu`'s `Mask` numbers them.
    pub fn of(n: i64) -> Mask {
        match n {
            1 => Mask::Write,
            2 => Mask::Inside,
            3 => Mask::Outside,
            _ => Mask::Off,
        }
    }
}

/// The form of a pipeline for one place and one way of drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Form {
    pub format: wgpu::TextureFormat,
    pub samples: u32,
    pub premultiplied: bool,
    pub mask: Mask,
}

impl Form {
    /// The plain form for a place of `format` and `samples`.
    pub(crate) fn plain(format: wgpu::TextureFormat, samples: u32) -> Form {
        Form {
            format,
            samples,
            premultiplied: false,
            mask: Mask::Off,
        }
    }
}

pub(crate) struct Made {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) shared: Option<wgpu::BindGroupLayout>,
    pub(crate) textures: Option<wgpu::BindGroupLayout>,
}

struct Pipeline {
    wgsl: String,
    layout: Layout,
    made: HashMap<Form, Arc<Made>>,
}

#[derive(Default)]
struct Pipelines {
    /// The places' formats and samples opened so far.
    places: Vec<(wgpu::TextureFormat, u32)>,
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
/// text. It is compiled now for each place open.
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
    let places = p.places.clone();
    drop(p);
    for (format, samples) in places {
        made(n, Form::plain(format, samples));
    }
    Ok(n)
}

/// Compiles every pipeline made so far for a place of `format` and
/// `samples`, and those made later as they come, so frames never wait
/// for a compile.
pub(crate) fn ready_for(format: wgpu::TextureFormat, samples: u32) {
    let mut p = pipelines();
    if p.places.contains(&(format, samples)) {
        return;
    }
    p.places.push((format, samples));
    let all: Vec<i64> = p.all.keys().copied().collect();
    drop(p);
    for n in all {
        made(n, Form::plain(format, samples));
    }
}

/// Whether pipeline `n` is compiled for some screen or picture yet.
pub fn is_made(n: i64) -> bool {
    pipelines().all.get(&n).is_some_and(|p| !p.made.is_empty())
}

/// Whether pipeline `n` draws with depth, so its target needs depths.
pub fn has_depth(n: i64) -> bool {
    pipelines().all.get(&n).is_some_and(|p| p.layout.depth)
}

/// Pipeline `n` compiled in `form`, and its layout.
pub(crate) fn made(n: i64, form: Form) -> Option<(Arc<Made>, Layout)> {
    let mut p = pipelines();
    let pipe = p.all.get_mut(&n)?;
    if let Some(m) = pipe.made.get(&form) {
        return Some((m.clone(), pipe.layout.clone()));
    }
    let m = Arc::new(compile(&pipe.wgsl, &pipe.layout, form)?);
    pipe.made.insert(form, m.clone());
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

/// The depths and mask a form keeps: nearer pixels only, each keeping
/// its depth (spec 15.3), and the mask marked or read (spec 14.24.1).
fn depth_stencil(layout: &Layout, mask: Mask) -> Option<wgpu::DepthStencilState> {
    if !layout.depth && mask == Mask::Off {
        return None;
    }
    let (compare, pass) = match mask {
        Mask::Off => (wgpu::CompareFunction::Always, wgpu::StencilOperation::Keep),
        Mask::Write => (
            wgpu::CompareFunction::Always,
            wgpu::StencilOperation::Replace,
        ),
        Mask::Inside => (wgpu::CompareFunction::Equal, wgpu::StencilOperation::Keep),
        Mask::Outside => (
            wgpu::CompareFunction::NotEqual,
            wgpu::StencilOperation::Keep,
        ),
    };
    let face = wgpu::StencilFaceState {
        compare,
        fail_op: wgpu::StencilOperation::Keep,
        depth_fail_op: wgpu::StencilOperation::Keep,
        pass_op: pass,
    };
    Some(wgpu::DepthStencilState {
        format: crate::texture::DEPTH,
        depth_write_enabled: Some(layout.depth),
        depth_compare: Some(if layout.depth {
            wgpu::CompareFunction::Less
        } else {
            wgpu::CompareFunction::Always
        }),
        stencil: wgpu::StencilState {
            front: face,
            back: face,
            read_mask: 0xFF,
            write_mask: if mask == Mask::Write { 0xFF } else { 0 },
        },
        bias: wgpu::DepthBiasState::default(),
    })
}

fn compile(wgsl: &str, layout: &Layout, form: Form) -> Option<Made> {
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
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
    let writing = form.mask == Mask::Write;
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
            depth_stencil: depth_stencil(layout, form.mask),
            multisample: wgpu::MultisampleState {
                count: form.samples,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                // Marking the mask leaves out the pixels of no opacity.
                entry_point: Some(if writing { "fs_mask" } else { "fs_main" }),
                compilation_options: Default::default(),
                // Opacity blends a pixel with what is drawn under it.
                targets: &[Some(wgpu::ColorTargetState {
                    format: form.format,
                    blend: Some(if form.premultiplied {
                        wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING
                    } else {
                        wgpu::BlendState::ALPHA_BLENDING
                    }),
                    write_mask: if writing {
                        wgpu::ColorWrites::empty()
                    } else {
                        wgpu::ColorWrites::ALL
                    },
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
