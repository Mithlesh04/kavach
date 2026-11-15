//! More parts of the machine on Linux (spec 14.25, decision 0112):
//! displays, the battery, sensors, devices, drivers and installed
//! packages, from `/sys`, `/proc` and dpkg's list. No `unsafe`.

use crate::parts::yes;
use std::fs::{read_dir, read_link, read_to_string};
use std::path::Path;

fn read(path: impl AsRef<Path>) -> String {
    read_to_string(path)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn names(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = (read_dir(dir).into_iter().flatten().flatten())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    out.sort();
    out
}

fn link_name(path: impl AsRef<Path>) -> String {
    (read_link(path).ok())
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default()
}

/// The port a display connector is: `card0-HDMI-A-1` is `HDMI`.
fn connection(connector: &str) -> &'static str {
    let c = connector.split_once('-').map_or(connector, |(_, c)| c);
    match c.split('-').next().unwrap_or("") {
        "HDMI" => "HDMI",
        "DP" => "DisplayPort",
        "eDP" | "LVDS" | "DSI" => "internal",
        "VGA" => "VGA",
        "DVI" => "DVI",
        _ => "",
    }
}

pub fn displays() -> Vec<Vec<String>> {
    (names("/sys/class/drm").into_iter())
        .filter(|n| n.contains('-') && read(format!("/sys/class/drm/{n}/status")) == "connected")
        .map(|n| {
            let mode = read(format!("/sys/class/drm/{n}/modes"));
            let (w, h) =
                (mode.lines().next().and_then(|m| m.split_once('x'))).unwrap_or(("0", "0"));
            let h: String = h.chars().take_while(char::is_ascii_digit).collect();
            vec![
                n.clone(),
                w.to_string(),
                h,
                "0".to_string(),
                connection(&n).to_string(),
            ]
        })
        .collect()
}

pub fn battery() -> Vec<Vec<String>> {
    let supplies = names("/sys/class/power_supply");
    let on_power = supplies.iter().any(|s| {
        let base = format!("/sys/class/power_supply/{s}");
        read(format!("{base}/type")) == "Mains" && read(format!("{base}/online")) == "1"
    });
    let Some(b) =
        (supplies.iter()).find(|s| read(format!("/sys/class/power_supply/{s}/type")) == "Battery")
    else {
        return Vec::new();
    };
    let base = format!("/sys/class/power_supply/{b}");
    let status = read(format!("{base}/status"));
    let charging = status == "Charging";
    let number = |f: &str| read(format!("{base}/{f}")).parse::<f64>().ok();
    // Energy (µWh) over power (µW), or charge (µAh) over current (µA).
    let left = match (
        number("energy_now"),
        number("power_now"),
        number("charge_now"),
        number("current_now"),
    ) {
        (Some(e), Some(p), ..) if p > 0.0 && !charging => (e / p * 60.0) as i64,
        (_, _, Some(c), Some(i)) if i > 0.0 && !charging => (c / i * 60.0) as i64,
        _ => -1,
    };
    vec![vec![
        read(format!("{base}/capacity")),
        yes(charging),
        yes(on_power || charging || status == "Full"),
        left.to_string(),
    ]]
}

pub fn sensors() -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for h in names("/sys/class/hwmon") {
        let base = format!("/sys/class/hwmon/{h}");
        let chip = read(format!("{base}/name"));
        for f in names(&base) {
            let Some(prefix) = f.strip_suffix("_input") else {
                continue;
            };
            let (kind, scale) = match prefix.trim_end_matches(|c: char| c.is_ascii_digit()) {
                "temp" => ("Temperature", 1000.0),
                "fan" => ("Fan", 1.0),
                "in" => ("Voltage", 1000.0),
                "power" => ("Power", 1_000_000.0),
                _ => continue,
            };
            let Ok(raw) = read(format!("{base}/{f}")).parse::<f64>() else {
                continue;
            };
            let label = read(format!("{base}/{prefix}_label"));
            let name = if label.is_empty() {
                format!("{chip} {prefix}")
            } else {
                format!("{chip} {label}")
            };
            out.push(vec![name, kind.to_string(), (raw / scale).to_string()]);
        }
    }
    out
}

/// A device's driver and the driver module's version.
fn driver_of(device: &str) -> (String, String) {
    let driver = link_name(format!("{device}/driver"));
    let version = read(format!("/sys/module/{driver}/version"));
    (driver, version)
}

pub fn devices() -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut add = |kind: &str, name: String, maker: String, device: String| {
        let (driver, version) = driver_of(&device);
        out.push(vec![
            kind.to_string(),
            name,
            maker,
            driver,
            version,
            yes(true),
        ]);
    };
    for n in names("/sys/class/bluetooth") {
        add(
            "bluetooth",
            n.clone(),
            String::new(),
            format!("/sys/class/bluetooth/{n}/device"),
        );
    }
    for n in names("/sys/class/net").into_iter().filter(|n| n != "lo") {
        add(
            "network",
            n.clone(),
            String::new(),
            format!("/sys/class/net/{n}/device"),
        );
    }
    for n in names("/sys/class/sound")
        .into_iter()
        .filter(|n| n.starts_with("card"))
    {
        let id = read(format!("/sys/class/sound/{n}/id"));
        add(
            "audio",
            if id.is_empty() { n.clone() } else { id },
            String::new(),
            format!("/sys/class/sound/{n}/device"),
        );
    }
    for d in displays() {
        let card = d[0].split('-').next().unwrap_or("").to_string();
        add(
            "display",
            d[0].clone(),
            String::new(),
            format!("/sys/class/drm/{card}/device"),
        );
    }
    for n in names("/sys/class/video4linux") {
        add(
            "camera",
            read(format!("/sys/class/video4linux/{n}/name")),
            String::new(),
            format!("/sys/class/video4linux/{n}/device"),
        );
    }
    for n in names("/sys/class/block") {
        if Path::new(&format!("/sys/block/{n}")).exists()
            && !n.starts_with("loop")
            && !n.starts_with("ram")
        {
            add(
                "storage",
                read(format!("/sys/block/{n}/device/model")),
                read(format!("/sys/block/{n}/device/vendor")),
                format!("/sys/block/{n}/device"),
            );
        }
    }
    // Keyboards and mice, by the handlers the input layer gives them.
    for block in read("/proc/bus/input/devices").split("\n\n") {
        let field = |p: &str| {
            (block.lines().find_map(|l| l.strip_prefix(p)))
                .unwrap_or("")
                .to_string()
        };
        let name = field("N: Name=").trim_matches('"').to_string();
        let handlers = field("H: Handlers=");
        let kind = if handlers.contains("kbd") && handlers.contains("leds") {
            "keyboard"
        } else if handlers.contains("mouse") {
            "mouse"
        } else {
            continue;
        };
        out.push(vec![
            kind.to_string(),
            name,
            String::new(),
            String::new(),
            String::new(),
            yes(true),
        ]);
    }
    for n in names("/sys/bus/usb/devices") {
        let base = format!("/sys/bus/usb/devices/{n}");
        let product = read(format!("{base}/product"));
        if n.contains(':') || product.is_empty() {
            continue;
        }
        // Hubs are the ports themselves, not devices plugged in.
        if read(format!("{base}/bDeviceClass")) == "09" {
            continue;
        }
        out.push(vec![
            "usb".to_string(),
            product,
            read(format!("{base}/manufacturer")),
            String::new(),
            String::new(),
            yes(true),
        ]);
    }
    out
}

/// Every driver registered on a bus, built in or loaded as a module.
pub fn drivers() -> Vec<Vec<String>> {
    let mut all: Vec<String> = (names("/sys/bus").iter())
        .flat_map(|bus| names(&format!("/sys/bus/{bus}/drivers")))
        .collect();
    all.sort();
    all.dedup();
    (all.into_iter())
        .map(|d| vec![d.clone(), read(format!("/sys/module/{d}/version"))])
        .collect()
}

/// Installed packages, from dpkg's list (Debian, Ubuntu and the like).
pub fn apps() -> Vec<Vec<String>> {
    let status = read("/var/lib/dpkg/status");
    (status.split("\n\n"))
        .filter(|p| p.lines().any(|l| l == "Status: install ok installed"))
        .map(|p| {
            let field = |f: &str| {
                (p.lines().find_map(|l| l.strip_prefix(f)))
                    .unwrap_or("")
                    .to_string()
            };
            vec![
                field("Package: "),
                field("Version: "),
                field("Maintainer: "),
            ]
        })
        .collect()
}
