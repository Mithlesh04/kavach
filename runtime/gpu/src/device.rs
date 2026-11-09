//! Opening the program's one GPU device (spec 15.2), through Vulkan
//! first.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};

/// The program's GPU: its device and queue, the layout every kernel's
/// buffers follow, and the kernels compiled so far.
pub(crate) struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) layout: wgpu::BindGroupLayout,
    /// The same with the items writable: a `map` that writes its results
    /// over them.
    pub(crate) in_place: wgpu::BindGroupLayout,
    pub(crate) pipelines: Mutex<HashMap<String, Arc<wgpu::ComputePipeline>>>,
    /// The most items one dispatch's buffers hold.
    pub(crate) max_items: usize,
}

static GPU: OnceLock<Option<Gpu>> = OnceLock::new();

/// Runs `f` to the end. `wgpu`'s native futures finish when polled, so
/// no executor is needed.
fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The GPU on one of `backends`, the fastest first.
fn adapter(backends: wgpu::Backends) -> Option<wgpu::Adapter> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    };
    block_on(instance.request_adapter(&options)).ok()
}

/// Opens the GPU, if there is one.
fn open() -> Option<Gpu> {
    // Vulkan alone opens in half the time of probing every backend; Metal
    // (Apple) and DirectX 12 are tried only when it finds no GPU.
    let adapter = adapter(wgpu::Backends::VULKAN).or_else(|| adapter(wgpu::Backends::PRIMARY))?;
    let limits = adapter.limits();
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("kavach"),
        required_limits: limits.clone(),
        ..Default::default()
    };
    let (device, queue) = block_on(adapter.request_device(&descriptor)).ok()?;
    let storage = |binding, read_only| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let layout = |items_read_only| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kavach map"),
            entries: &[
                storage(0, items_read_only),
                storage(1, false),
                storage(2, true),
                storage(3, false),
            ],
        })
    };
    let (layout, in_place) = (layout(true), layout(false));
    let words = limits
        .max_storage_buffer_binding_size
        .min(limits.max_buffer_size)
        / 4;
    Some(Gpu {
        device,
        queue,
        layout,
        in_place,
        pipelines: Mutex::new(HashMap::new()),
        max_items: words as usize,
    })
}

/// The program's GPU, opened at the first call.
pub(crate) fn gpu() -> Option<&'static Gpu> {
    GPU.get_or_init(open).as_ref()
}

/// Whether this machine has a GPU `map` can use.
pub fn available() -> bool {
    gpu().is_some()
}

/// Whether the GPU is open and ready now, without waiting for it.
pub fn ready() -> bool {
    GPU.get().is_some_and(Option::is_some)
}
