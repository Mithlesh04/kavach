//! Compiling a kernel into a pipeline, once per kernel text (spec 15.2).
//! A kernel whose loops surely end goes without `wgpu`'s counter in each
//! loop (decision 0077).

use crate::device::Gpu;
use crate::{ITEMS, ITEMS_WRITTEN};
use std::sync::Arc;

/// Whether kernel `wgsl` says every loop in it surely ends (decision 0077):
/// a line of its header, before the code.
fn loops_end(wgsl: &str) -> bool {
    wgsl.lines()
        .take_while(|l| l.starts_with("//"))
        .any(|l| l == "// loops end")
}

impl Gpu {
    /// The pipeline of kernel `wgsl`, compiled at its first use; with
    /// `in_place`, the kernel's items are writable, as its results are
    /// written over them.
    pub(crate) fn pipeline(&self, wgsl: &str, in_place: bool) -> Arc<wgpu::ComputePipeline> {
        let text = match in_place {
            true => wgsl.replacen(ITEMS, ITEMS_WRITTEN, 1),
            false => wgsl.to_string(),
        };
        let mut cache = self.pipelines.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(p) = cache.get(&text) {
            return p.clone();
        }
        let source = wgpu::ShaderModuleDescriptor {
            label: Some("kavach kernel"),
            source: wgpu::ShaderSource::Wgsl(text.as_str().into()),
        };
        let module = if loops_end(&text) {
            let mut checks = wgpu::ShaderRuntimeChecks::checked();
            checks.force_loop_bounding = false;
            // SAFETY: `wgpu` asks that every loop of the kernel ends, which
            // the compiler's `// loops end` line says: each loop is a `for`
            // over an exclusive range (decision 0077), and only the compiler
            // writes kernel text. Every other check stays on.
            #[allow(unsafe_code)]
            unsafe {
                self.device.create_shader_module_trusted(source, checks)
            }
        } else {
            self.device.create_shader_module(source)
        };
        let group = if in_place {
            &self.in_place
        } else {
            &self.layout
        };
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("kavach map"),
                bind_group_layouts: &[Some(group)],
                immediate_size: 0,
            });
        let pipeline = Arc::new(self.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                label: Some("kavach kernel"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            },
        ));
        cache.insert(text, pipeline.clone());
        pipeline
    }
}
