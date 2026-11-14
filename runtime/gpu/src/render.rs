//! Pipelines and draws (spec 15.3, decision 0106). A pipeline is its WGSL
//! module, written by the compiler (`compiler/backend/wgsl/src/render.rs`),
//! whose first line gives the buffers' layout: `// draw f f | f f f | 1`,
//! the vertex's fields, the instance's, and the shared value's words. It
//! is compiled at its first draw into a frame of each color format.

use crate::device::gpu;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use wgpu::util::DeviceExt;

/// What the first line of a pipeline's module says.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub vertex: Vec<wgpu::VertexFormat>,
    pub instance: Vec<wgpu::VertexFormat>,
    pub shared: usize,
}

/// The layout the first line of `wgsl` gives.
pub fn layout(wgsl: &str) -> Option<Layout> {
    let line = wgsl.lines().next()?.strip_prefix("// draw ")?;
    let parts: Vec<&str> = line.split('|').map(str::trim).collect();
    let [vertex, instance, shared] = parts[..] else {
        return None;
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
    })
}

struct Made {
    pipeline: wgpu::RenderPipeline,
    shared: Option<wgpu::BindGroupLayout>,
}

struct Pipeline {
    wgsl: String,
    layout: Layout,
    /// Compiled for each color format it has drawn into.
    made: HashMap<wgpu::TextureFormat, Arc<Made>>,
}

#[derive(Default)]
struct Pipelines {
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
/// text. Compiling waits for its first draw.
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
    Ok(n)
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
    let groups: Vec<Option<&wgpu::BindGroupLayout>> = shared.iter().map(Some).collect();
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
            depth_stencil: None,
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
    Some(Made { pipeline, shared })
}

/// Draws `vertices` (32-bit words, the vertex's fields in turn) through
/// pipeline `n` into `view`, recorded on `encoder`: once for each of
/// `instances` when the pipeline takes an instance, with `shared` given
/// to every corner. What is there already stays under it.
pub(crate) fn draw_into(
    view: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
    format: wgpu::TextureFormat,
    n: i64,
    parts: [&[u32]; 3],
) -> Result<(), String> {
    let [vertices, instances, shared] = parts;
    let g = gpu().ok_or("this machine has no GPU")?;
    let (m, layout) = made(n, format).ok_or("this pipeline couldn't be made")?;
    let width = layout.vertex.len().max(1);
    let count = (vertices.len() / width) as u32;
    let buffer = |words: &[u32], usage| {
        g.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("kavach draw"),
                contents: &bytes_of(if words.is_empty() { &[0] } else { words }),
                usage,
            })
    };
    let vb = buffer(vertices, wgpu::BufferUsages::VERTEX);
    let ib = (!layout.instance.is_empty()).then(|| buffer(instances, wgpu::BufferUsages::VERTEX));
    let times = match &ib {
        Some(_) => (instances.len() / layout.instance.len()) as u32,
        None => 1,
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
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    pass.set_pipeline(&m.pipeline);
    if let Some(group) = &group {
        pass.set_bind_group(0, group, &[]);
    }
    pass.set_vertex_buffer(0, vb.slice(..));
    if let Some(ib) = &ib {
        pass.set_vertex_buffer(1, ib.slice(..));
    }
    pass.draw(0..count, 0..times);
    Ok(())
}

/// Words as bytes, little-endian as GPUs read them.
fn bytes_of(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// Draws through pipeline `n` into a new picture of `size` pixels, cleared
/// to black, and gives its pixels: red, green, blue and opacity, row by
/// row from the top. Drawing into a texture, for tests and, later, for
/// programs (decision 0106).
pub fn picture(n: i64, parts: [&[u32]; 3], size: (u32, u32)) -> Result<Vec<[u8; 4]>, String> {
    let g = gpu().ok_or("this machine has no GPU")?;
    let (w, h) = (size.0.max(1), size.1.max(1));
    let format = wgpu::TextureFormat::Rgba8Unorm;
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
    draw_into(&view, &mut encoder, format, n, parts)?;
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
