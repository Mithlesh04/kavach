// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Opening one GPU device (spec 15.2, 15.6): its queue, the layout every
//! kernel's buffers follow, and a watch for its loss. Which device runs
//! the work is chosen in `devices.rs`.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// A GPU device: its queue, the layout every kernel's buffers follow,
/// and the kernels compiled so far.
pub(crate) struct Gpu {
    /// Its number, as `system.gpus()` lists the devices (spec 15.6).
    pub(crate) index: usize,
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
    /// The work sent to it and not finished: Kavach's own count, which
    /// sends new work to the least busy device.
    pub(crate) busy: AtomicUsize,
    /// Set when the device is lost or runs out of memory: it is skipped
    /// from then on.
    pub(crate) lost: Arc<AtomicBool>,
}

/// Work sent to a device and not finished: the device's `busy` counts it
/// until this is let go.
pub(crate) struct Pending(&'static Gpu);

impl Pending {
    pub(crate) fn new(gpu: &'static Gpu) -> Pending {
        gpu.busy.fetch_add(1, Ordering::Relaxed);
        Pending(gpu)
    }

    /// The device the work runs on.
    pub(crate) fn gpu(&self) -> &'static Gpu {
        self.0
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.0.busy.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Gpu {
    /// Whether the device was lost or ran out of memory.
    pub(crate) fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Relaxed)
    }
}

/// Runs `f` to the end. `wgpu`'s native futures finish when polled, so
/// no executor is needed.
pub(crate) fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// Opens `adapter` as device number `index`, or `None` with why not.
pub(crate) fn open(
    index: usize,
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
) -> Result<Gpu, String> {
    let limits = adapter.limits();
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("kavach"),
        required_limits: limits.clone(),
        // Timestamps, when a trace is written (spec 14.12), and the block
        // formats of compressed textures the GPU reads (spec 14.24).
        required_features: crate::timers::features(&adapter)
            | (adapter.features()
                & (wgpu::Features::TEXTURE_COMPRESSION_BC
                    | wgpu::Features::TEXTURE_COMPRESSION_ASTC)),
        ..Default::default()
    };
    let (device, queue) =
        block_on(adapter.request_device(&descriptor)).map_err(|e| e.to_string())?;
    // A lost device, or one out of memory, is skipped from then on (spec
    // 15.6); any other error is a mistake in Kavach, and stops it.
    let lost = Arc::new(AtomicBool::new(false));
    let on_lost = lost.clone();
    device.set_device_lost_callback(move |_, _| on_lost.store(true, Ordering::Relaxed));
    let on_error = lost.clone();
    device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| match e {
        wgpu::Error::OutOfMemory { .. } => on_error.store(true, Ordering::Relaxed),
        e => panic!("the GPU reported an error: {e}"),
    }));
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
    Ok(Gpu {
        index,
        instance,
        adapter,
        device,
        queue,
        layout,
        in_place,
        pipelines: Mutex::new(HashMap::new()),
        max_items: words as usize,
        busy: AtomicUsize::new(0),
        lost,
    })
}
