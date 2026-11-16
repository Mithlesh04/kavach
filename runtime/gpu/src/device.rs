//! Opening the program's one GPU device (spec 15.2), through Vulkan
//! first.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};

/// The program's GPU: its device and queue, the layout every kernel's
/// buffers follow, and the kernels compiled so far.
pub(crate) struct Gpu {
    /// The instance and adapter the device came from: a window's surface
    /// is made from the same ones (spec 14.24).
    pub(crate) instance: wgpu::Instance,
    pub(crate) adapter: wgpu::Adapter,
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

/// The GPU on one of `backends`, the fastest first, with its instance.
fn adapter(backends: wgpu::Backends) -> Option<(wgpu::Instance, wgpu::Adapter)> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let options = wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    };
    let adapter = block_on(instance.request_adapter(&options)).ok()?;
    Some((instance, adapter))
}

/// Opens the GPU, if there is one.
fn open() -> Option<Gpu> {
    // Vulkan alone opens in half the time of probing every backend; Metal
    // (Apple) and DirectX 12 are tried only when it finds no GPU.
    let (instance, adapter) =
        adapter(wgpu::Backends::VULKAN).or_else(|| adapter(wgpu::Backends::PRIMARY))?;
    let limits = adapter.limits();
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("kavach"),
        required_limits: limits.clone(),
        // Timestamps, when a trace is written (spec 14.12).
        required_features: crate::timers::features(&adapter),
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
        instance,
        adapter,
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

/// Every GPU this machine has (spec 14.25): name, vendor, kind and the
/// interface it is reached through, discrete ones first, then integrated,
/// the rest last. Each appears once, through the first interface that
/// lists it; nothing is opened.
pub fn adapters() -> Vec<[String; 4]> {
    let mut found: Vec<(u8, [String; 4])> = Vec::new();
    for backends in [
        wgpu::Backends::VULKAN,
        wgpu::Backends::METAL,
        wgpu::Backends::DX12,
        wgpu::Backends::GL,
    ] {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);
        for a in block_on(instance.enumerate_adapters(backends)) {
            let info = a.get_info();
            if found.iter().any(|(_, f)| f[0] == info.name) {
                continue;
            }
            let (rank, kind) = match info.device_type {
                wgpu::DeviceType::DiscreteGpu => (0, "Discrete"),
                wgpu::DeviceType::IntegratedGpu => (1, "Integrated"),
                wgpu::DeviceType::VirtualGpu => (2, "Virtual"),
                wgpu::DeviceType::Other => (3, "Other"),
                wgpu::DeviceType::Cpu => (4, "Software"),
            };
            let vendor = match info.vendor {
                0x10de => "NVIDIA",
                0x1002 | 0x1022 => "AMD",
                0x8086 => "Intel",
                0x106b => "Apple",
                0x5143 => "Qualcomm",
                0x13b5 => "ARM",
                0x1414 => "Microsoft",
                _ => "",
            };
            let backend = match info.backend {
                wgpu::Backend::Vulkan => "vulkan",
                wgpu::Backend::Metal => "metal",
                wgpu::Backend::Dx12 => "dx12",
                wgpu::Backend::Gl => "gl",
                _ => "",
            };
            let texts = [
                info.name.clone(),
                vendor.into(),
                kind.into(),
                backend.into(),
            ];
            found.push((rank, texts));
        }
    }
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, texts)| texts).collect()
}
