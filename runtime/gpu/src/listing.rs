// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! Listing every GPU the machine has (spec 14.25), best first, without
//! opening one. The vendor names, kinds and ranking are plain functions,
//! so they are tested on any machine; `KAVACH_FAKE_GPUS` replaces the
//! real list, so `system.gpus()` is tested with every vendor's GPU too.

/// What kind of GPU an adapter is, best first: a sort by kind puts
/// discrete GPUs before integrated ones, and software last.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Discrete,
    Integrated,
    Virtual,
    Other,
    Software,
}

impl Kind {
    /// The kind of a `wgpu` device type.
    pub(crate) fn of(t: wgpu::DeviceType) -> Kind {
        match t {
            wgpu::DeviceType::DiscreteGpu => Kind::Discrete,
            wgpu::DeviceType::IntegratedGpu => Kind::Integrated,
            wgpu::DeviceType::VirtualGpu => Kind::Virtual,
            wgpu::DeviceType::Other => Kind::Other,
            wgpu::DeviceType::Cpu => Kind::Software,
        }
    }

    /// The name `std.system` turns into a `GpuKind`.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Discrete => "Discrete",
            Kind::Integrated => "Integrated",
            Kind::Virtual => "Virtual",
            Kind::Other => "Other",
            Kind::Software => "Software",
        }
    }

    /// A kind written in any case, as in `KAVACH_FAKE_GPUS`.
    fn parse(text: &str) -> Option<Kind> {
        let all = [
            Kind::Discrete,
            Kind::Integrated,
            Kind::Virtual,
            Kind::Other,
            Kind::Software,
        ];
        all.into_iter()
            .find(|k| k.name().eq_ignore_ascii_case(text))
    }
}

/// The maker of a GPU, from its PCI vendor ID; `""` for one not known.
pub(crate) fn vendor(id: u32) -> &'static str {
    match id {
        0x10de => "NVIDIA",
        0x1002 | 0x1022 => "AMD",
        0x8086 => "Intel",
        0x106b => "Apple",
        0x5143 => "Qualcomm",
        0x13b5 => "ARM",
        0x1414 => "Microsoft",
        _ => "",
    }
}

/// The name of the interface a GPU is reached through.
fn backend(b: wgpu::Backend) -> &'static str {
    match b {
        wgpu::Backend::Vulkan => "vulkan",
        wgpu::Backend::Metal => "metal",
        wgpu::Backend::Dx12 => "dx12",
        wgpu::Backend::Gl => "gl",
        _ => "",
    }
}

/// One GPU as an interface lists it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Listed {
    pub(crate) name: String,
    pub(crate) vendor: u32,
    pub(crate) kind: Kind,
    pub(crate) backend: String,
    /// Where it is on the PCI bus, when the interface says (`""` if not):
    /// two GPUs of one model are told apart by it.
    pub(crate) bus: String,
    /// The model's PCI device ID, 0 when not known: one GPU may have
    /// another name through another interface.
    pub(crate) model: u32,
}

impl Listed {
    /// Its name, vendor, kind and interface, as `system.gpus()` gives them.
    pub(crate) fn info(&self) -> [String; 4] {
        [
            self.name.clone(),
            vendor(self.vendor).into(),
            self.kind.name().into(),
            self.backend.clone(),
        ]
    }

    /// Whether `other` is this GPU listed again through another
    /// interface: the same vendor and model (or the same name, when a
    /// model isn't known), and the same bus where both say.
    fn again(&self, other: &Listed) -> bool {
        let same = match (self.model, other.model) {
            (0, _) | (_, 0) => self.name == other.name,
            (a, b) => a == b && self.vendor == other.vendor,
        };
        same && self.backend != other.backend
            && (self.bus == other.bus || self.bus.is_empty() || other.bus.is_empty())
    }
}

/// The GPUs best first: discrete, integrated, virtual, other, software,
/// each kind in the order they were listed. A GPU listed again through
/// another interface keeps only its first. `of` gives each item's GPU.
pub(crate) fn ranked<T>(listed: Vec<T>, of: impl Fn(&T) -> &Listed) -> Vec<T> {
    let mut once: Vec<T> = Vec::new();
    for g in listed {
        if !once.iter().any(|o| of(o).again(of(&g))) {
            once.push(g);
        }
    }
    // A stable sort: ties keep the order they were listed in.
    once.sort_by_key(|g| of(g).kind);
    once
}

/// The GPUs `KAVACH_FAKE_GPUS` names, for tests: entries split by `;`,
/// each `name:vendor:kind:backend`, the vendor a PCI ID (`0x1002`, or
/// decimal) and the kind's name in any case. The name may hold `:`. An
/// entry that doesn't read so is left out; an empty text lists none.
pub(crate) fn fake(text: &str) -> Vec<Listed> {
    let entry = |e: &str| -> Option<Listed> {
        let mut parts = e.rsplitn(4, ':');
        let backend = parts.next()?.trim();
        let kind = Kind::parse(parts.next()?.trim())?;
        let id = parts.next()?.trim();
        let name = parts.next()?.trim();
        let vendor = match id.strip_prefix("0x").or_else(|| id.strip_prefix("0X")) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => id.parse().ok()?,
        };
        Some(Listed {
            name: name.into(),
            vendor,
            kind,
            backend: backend.to_ascii_lowercase(),
            bus: String::new(),
            model: 0,
        })
    };
    text.split(';').filter_map(entry).collect()
}

/// A GPU the machine has, and how to open it.
pub(crate) type Real = (Listed, wgpu::Instance, wgpu::Adapter);

/// Every GPU through Vulkan, Metal, DirectX 12 and OpenGL, in that
/// order. Nothing is opened.
pub(crate) fn real() -> Vec<Real> {
    let mut listed = Vec::new();
    for backends in [
        wgpu::Backends::VULKAN,
        wgpu::Backends::METAL,
        wgpu::Backends::DX12,
        wgpu::Backends::GL,
    ] {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = backends;
        let instance = wgpu::Instance::new(desc);
        for a in crate::device::block_on(instance.enumerate_adapters(backends)) {
            let info = a.get_info();
            let gpu = Listed {
                name: info.name,
                vendor: info.vendor,
                kind: Kind::of(info.device_type),
                backend: backend(info.backend).into(),
                bus: info.device_pci_bus_id,
                model: info.device,
            };
            listed.push((gpu, instance.clone(), a));
        }
    }
    listed
}

/// Every GPU this machine has (spec 14.25): name, vendor, kind and the
/// interface it is reached through, discrete ones first, then integrated,
/// the rest last, numbered as the program's devices are (15.6). Each
/// appears once, through the first interface that lists it; nothing is
/// opened. With `KAVACH_FAKE_GPUS` set, its list stands in for the
/// machine's.
pub fn adapters() -> Vec<[String; 4]> {
    (crate::devices::table().iter())
        .map(|d| d.listed.info())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vendor_id_has_its_maker() {
        let table = [
            (0x10de, "NVIDIA"),
            (0x1002, "AMD"),
            (0x1022, "AMD"),
            (0x8086, "Intel"),
            (0x106b, "Apple"),
            (0x5143, "Qualcomm"),
            (0x13b5, "ARM"),
            (0x1414, "Microsoft"),
            (0x0000, ""),
            (0x1234, ""),
        ];
        for (id, maker) in table {
            assert_eq!(vendor(id), maker, "vendor {id:#06x}");
        }
    }

    #[test]
    fn every_device_type_has_its_kind_and_name() {
        let table = [
            (wgpu::DeviceType::DiscreteGpu, Kind::Discrete, "Discrete"),
            (
                wgpu::DeviceType::IntegratedGpu,
                Kind::Integrated,
                "Integrated",
            ),
            (wgpu::DeviceType::VirtualGpu, Kind::Virtual, "Virtual"),
            (wgpu::DeviceType::Other, Kind::Other, "Other"),
            (wgpu::DeviceType::Cpu, Kind::Software, "Software"),
        ];
        for (t, kind, name) in table {
            assert_eq!(Kind::of(t), kind);
            assert_eq!(kind.name(), name);
            assert_eq!(Kind::parse(&name.to_lowercase()), Some(kind));
        }
        assert_eq!(Kind::parse("gpu"), None);
    }

    pub(crate) fn gpu(name: &str, vendor: u32, kind: Kind) -> Listed {
        let backend = "vulkan".into();
        Listed {
            name: name.into(),
            vendor,
            kind,
            backend,
            bus: String::new(),
            model: 0,
        }
    }

    #[test]
    fn the_best_kind_comes_first_and_ties_keep_their_order() {
        let listed = vec![
            gpu("soft", 0x1414, Kind::Software),
            gpu("other", 0, Kind::Other),
            gpu("igpu a", 0x8086, Kind::Integrated),
            gpu("virtual", 0x1af4, Kind::Virtual),
            gpu("dgpu a", 0x1002, Kind::Discrete),
            gpu("igpu b", 0x1002, Kind::Integrated),
            gpu("dgpu b", 0x10de, Kind::Discrete),
        ];
        let names: Vec<String> = ranked(listed, |g| g).into_iter().map(|g| g.name).collect();
        let order = [
            "dgpu a", "dgpu b", "igpu a", "igpu b", "virtual", "other", "soft",
        ];
        assert_eq!(names, order);
    }

    #[test]
    fn a_gpu_listed_twice_keeps_its_first_interface() {
        let mut again = gpu("dgpu", 0x10de, Kind::Discrete);
        again.backend = "dx12".into();
        let listed = vec![gpu("dgpu", 0x10de, Kind::Discrete), again];
        let want = ["dgpu", "NVIDIA", "Discrete", "vulkan"].map(String::from);
        let got: Vec<_> = ranked(listed, |g| g).iter().map(Listed::info).collect();
        assert_eq!(got, vec![want]);
    }

    #[test]
    fn two_gpus_of_one_model_are_two_devices() {
        // Each GPU listed, as (name, interface, bus), and which are kept.
        let table = [
            (vec![("a", "vulkan", "1"), ("a", "vulkan", "2")], vec![0, 1]),
            (vec![("a", "vulkan", "1"), ("a", "dx12", "")], vec![0]),
            (vec![("a", "vulkan", "1"), ("a", "dx12", "2")], vec![0, 1]),
            (vec![("a", "vulkan", ""), ("a", "vulkan", "")], vec![0, 1]),
            (vec![("a", "vulkan", ""), ("a", "dx12", "7")], vec![0]),
            (vec![("a", "vulkan", "1"), ("b", "dx12", "1")], vec![0, 1]),
        ];
        // One GPU named apart by two interfaces: its model tells.
        let mut vk = gpu("Intel(R) UHD Graphics 770", 0x8086, Kind::Integrated);
        let mut dx = gpu("Intel(R) UHD Graphics", 0x8086, Kind::Integrated);
        dx.backend = "dx12".into();
        (vk.model, dx.model) = (0x4680, 0x4680);
        let once = ranked(vec![vk.clone(), dx.clone()], |g| g);
        assert_eq!(once, vec![vk.clone()]);
        // Two models of one vendor stay two.
        dx.model = 0x4682;
        assert_eq!(ranked(vec![vk.clone(), dx.clone()], |g| g).len(), 2);
        for (listed, kept) in table {
            let gpus: Vec<(usize, Listed)> = (listed.iter().enumerate())
                .map(|(i, (name, backend, bus))| {
                    let mut g = gpu(name, 0x10de, Kind::Discrete);
                    g.backend = backend.to_string();
                    g.bus = bus.to_string();
                    (i, g)
                })
                .collect();
            let got: Vec<usize> = ranked(gpus, |g| &g.1).iter().map(|g| g.0).collect();
            assert_eq!(got, kept, "{listed:?}");
        }
    }

    #[test]
    fn a_fake_list_reads_each_entry_and_leaves_out_bad_ones() {
        let text = "AMD Radeon RX 7900:0x1002:discrete:vulkan; Intel UHD 770 : 0x8086 : Integrated : DX12;\
                    no kind:0x10de:fast:vulkan;bad id:0xzz:discrete:gl;short:discrete;;\
                    a:b name:4318:discrete:metal";
        let want = vec![
            Listed {
                name: "AMD Radeon RX 7900".into(),
                vendor: 0x1002,
                kind: Kind::Discrete,
                backend: "vulkan".into(),
                bus: String::new(),
                model: 0,
            },
            Listed {
                name: "Intel UHD 770".into(),
                vendor: 0x8086,
                kind: Kind::Integrated,
                backend: "dx12".into(),
                bus: String::new(),
                model: 0,
            },
            Listed {
                name: "a:b name".into(),
                vendor: 0x10de,
                kind: Kind::Discrete,
                backend: "metal".into(),
                bus: String::new(),
                model: 0,
            },
        ];
        assert_eq!(fake(text), want);
        assert!(fake("").is_empty());
    }
}
