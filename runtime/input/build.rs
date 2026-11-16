//! SDL3 is built from its source with CMake (decision 0124). Without
//! CMake, stop here and say how to get it on this system, instead of
//! failing deep inside SDL's build.

use std::process::Command;

fn main() {
    println!("cargo::rerun-if-env-changed=CMAKE");
    let cmake = std::env::var("CMAKE").unwrap_or_else(|_| "cmake".into());
    if Command::new(&cmake)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        return;
    }
    let how = match std::env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("windows") => "winget install Kitware.CMake".to_string(),
        Ok("macos") => "brew install cmake".to_string(),
        _ => linux_command(),
    };
    panic!(
        "\n\nBuilding Kavach needs CMake, to build SDL3 for gamepads (decision 0124), and it isn't installed.\nInstall it with:\n\n    {how}\n\nthen build again. (Or set CMAKE to the path of a cmake that isn't on PATH.)\n\n"
    );
}

/// The command that installs CMake on this Linux, from its package
/// manager.
fn linux_command() -> String {
    let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let like = |names: &[&str]| {
        release.lines().any(|l| {
            (l.starts_with("ID=") || l.starts_with("ID_LIKE="))
                && names.iter().any(|n| l.contains(n))
        })
    };
    if like(&["debian", "ubuntu"]) {
        "sudo apt install cmake".into()
    } else if like(&["fedora", "rhel", "centos"]) {
        "sudo dnf install cmake".into()
    } else if like(&["arch"]) {
        "sudo pacman -S cmake".into()
    } else if like(&["suse"]) {
        "sudo zypper install cmake".into()
    } else {
        "your package manager's `cmake` package, or https://cmake.org/download/".into()
    }
}
