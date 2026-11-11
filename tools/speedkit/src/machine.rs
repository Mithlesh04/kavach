//! What the speed kit says about the machine it ran on: the system, the
//! processor and how many threads it has.

use std::process::Command;

/// A few lines about this machine.
pub fn describe() -> Vec<String> {
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    vec![
        format!(
            "machine: {} {}, {threads} threads",
            std::env::consts::OS,
            std::env::consts::ARCH
        ),
        format!(
            "processor: {}",
            processor().unwrap_or_else(|| "unknown".into())
        ),
    ]
}

/// The processor's name, as the system reports it.
fn processor() -> Option<String> {
    if cfg!(target_os = "linux") {
        let info = std::fs::read_to_string("/proc/cpuinfo").ok()?;
        let line = info.lines().find(|l| l.starts_with("model name"))?;
        return Some(line.split_once(':')?.1.trim().to_string());
    }
    if cfg!(target_os = "macos") {
        let out = Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()?;
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    if cfg!(windows) {
        // The registry holds the marketing name; the environment only
        // the family and model numbers.
        let out = Command::new("reg")
            .args([
                "query",
                r"HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0",
                "/v",
                "ProcessorNameString",
            ])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.lines().find(|l| l.contains("ProcessorNameString"))?;
        let name = line.split("REG_SZ").nth(1)?.trim().to_string();
        return Some(name).filter(|n| !n.is_empty());
    }
    None
}
