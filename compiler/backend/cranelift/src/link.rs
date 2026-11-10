//! Linking an object file with the runtime library into an executable,
//! or a shared library (decision 0026), using the platform's linker
//! (decision 0016): MSVC `link.exe` on Windows, the C compiler driver
//! `cc` elsewhere.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The file name of the runtime library that cargo builds.
const RUNTIME_LIB: &str = if cfg!(windows) {
    "kavach_runtime_native.lib"
} else {
    "libkavach_runtime_native.a"
};

/// Where the runtime library is: `KAVACH_RUNTIME_LIB`, or next to the
/// `kavach` executable.
fn runtime_lib() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("KAVACH_RUNTIME_LIB") {
        return Ok(PathBuf::from(path));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("the kavach executable has no folder")?;
    // Next to the executable (an installed Kavach), or in a cargo build
    // folder's `deps/`, with a hash in the name. Several builds can leave
    // copies, so the newest one is the current build.
    let (stem, ext) = RUNTIME_LIB.split_once('.').unwrap_or((RUNTIME_LIB, ""));
    let in_deps = std::fs::read_dir(dir.join("deps"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p
                .file_name()
                .map_or(String::new(), |n| n.to_string_lossy().to_string());
            name.starts_with(&format!("{stem}-")) && name.ends_with(&format!(".{ext}"))
        });
    // An installed Kavach keeps it in `lib/` beside `bin/` (decision
    // 0080): `<root>/bin/kavach` with `<root>/lib/<RUNTIME_LIB>`.
    let in_lib = [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|root| root.join("lib").join(RUNTIME_LIB));
    let newest = std::iter::once(dir.join(RUNTIME_LIB))
        .chain(in_lib)
        .chain(in_deps)
        .filter(|p| p.is_file())
        .max_by_key(|p| p.metadata().and_then(|m| m.modified()).ok());
    if let Some(path) = newest {
        return Ok(path);
    }
    Err(format!(
        "can't find the runtime library `{RUNTIME_LIB}` next to `{}`; build it with `cargo build -p kavach-runtime-native` or set KAVACH_RUNTIME_LIB",
        exe.display()
    ))
}

/// Links `object` with the runtime library and the C libraries `libs`
/// (spec 10.7): `"c"` and `"m"` are always there; another `"z"` is `-lz`,
/// or `z.lib` on Windows.
/// The temporary folder a build of this process works in,
/// `<temp>/<prefix>-<pid>`. A build stopped half way (Ctrl+C) leaves its
/// folder behind, so the folders with this prefix older than a day go
/// first: their builds ended long ago.
pub fn build_folder(prefix: &str) -> PathBuf {
    let temp = std::env::temp_dir();
    remove_old(&temp, prefix, std::time::Duration::from_secs(86_400));
    temp.join(format!("{prefix}-{}", std::process::id()))
}

/// Removes the folders `<prefix>-<pid>` in `dir` older than `age`.
fn remove_old(dir: &Path, prefix: &str, age: std::time::Duration) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let ours = name
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('-'))
            .is_some_and(|pid| !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()));
        let elapsed = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok());
        if ours && elapsed.is_some_and(|e| e >= age) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

pub fn link(object: &Path, out: &Path, libs: &[String]) -> Result<(), String> {
    let libs: Vec<&String> = libs.iter().filter(|l| *l != "c" && *l != "m").collect();
    let lib = runtime_lib()?;
    let mut cmd = linker()?;
    if cfg!(windows) {
        cmd.arg("/NOLOGO")
            .arg("/SUBSYSTEM:CONSOLE")
            .arg(format!("/OUT:{}", out.display()))
            .arg(object)
            .arg(&lib)
            .args(WINDOWS_LIBS);
        cmd.args(libs.iter().map(|l| format!("{l}.lib")));
    } else {
        cmd.arg(object).arg(&lib).arg("-o").arg(out);
        cmd.args(libs.iter().map(|l| format!("-l{l}")));
        if cfg!(target_os = "linux") {
            // Drops the runtime code the program never calls, and the
            // runtime's debug information, as the Windows linker does
            // (decision 0024).
            cmd.args(["-Wl,--gc-sections", "-s", "-lpthread", "-ldl", "-lm"]);
        }
    }
    run(cmd)
}

/// What the Rust standard library inside the runtime needs on Windows,
/// and the system's random numbers for TLS (`bcrypt`, `advapi32`).
///
/// The C runtime's start and `vcruntime` are linked in (`libcmt`,
/// `libvcruntime`), so a program needs no `VCRUNTIME140.dll`, which a
/// fresh Windows may lack; the Universal CRT (`ucrt`) stays a DLL, as
/// every Windows from 10 on has it.
const WINDOWS_LIBS: [&str; 13] = [
    "kernel32.lib",
    "ntdll.lib",
    "userenv.lib",
    "ws2_32.lib",
    "dbghelp.lib",
    "/NODEFAULTLIB:msvcrt.lib",
    "/NODEFAULTLIB:vcruntime.lib",
    "/NODEFAULTLIB:libucrt.lib",
    "libcmt.lib",
    "libvcruntime.lib",
    "ucrt.lib",
    "bcrypt.lib",
    "advapi32.lib",
];

/// Links `object` with the runtime library into the shared library
/// `out`, exporting only `exports` (decision 0026): the runtime inside
/// stays hidden, so a program that loads the library keeps its own.
pub fn link_library(
    object: &Path,
    out: &Path,
    exports: &[&str],
    libs: &[String],
) -> Result<(), String> {
    let libs: Vec<&String> = libs.iter().filter(|l| *l != "c" && *l != "m").collect();
    let lib = runtime_lib()?;
    let mut cmd = linker()?;
    if cfg!(windows) {
        // A DLL exports only what `/EXPORT` names; next to it, the import
        // library (`.lib`) that programs link with.
        cmd.arg("/NOLOGO")
            .arg("/DLL")
            .arg(format!("/OUT:{}", out.display()))
            .arg(object)
            .arg(&lib)
            .args(WINDOWS_LIBS);
        cmd.args(exports.iter().map(|e| format!("/EXPORT:{e}")));
        cmd.args(libs.iter().map(|l| format!("{l}.lib")));
    } else {
        let shared = if cfg!(target_os = "macos") {
            "-dynamiclib"
        } else {
            "-shared"
        };
        cmd.arg(shared).arg(object).arg(&lib).arg("-o").arg(out);
        cmd.args(libs.iter().map(|l| format!("-l{l}")));
        if cfg!(target_os = "linux") {
            // The runtime's symbols become local to the library.
            cmd.args([
                "-Wl,--exclude-libs,ALL",
                "-Wl,--gc-sections",
                "-lpthread",
                "-ldl",
                "-lm",
            ]);
        }
    }
    run(cmd)
}

fn run(mut cmd: Command) -> Result<(), String> {
    let output = cmd.output().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => missing_linker(&cmd),
        _ => format!("can't run the linker: {e}"),
    })?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "the linker failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    ))
}

/// What to install when the linker `cmd` runs isn't there (decision
/// 0080: the linker is checked and named, not bundled).
fn missing_linker(cmd: &Command) -> String {
    let name = cmd.get_program().to_string_lossy();
    let install = if cfg!(windows) {
        "install the Visual Studio Build Tools with the C++ workload: https://visualstudio.microsoft.com/visual-cpp-build-tools/"
    } else if cfg!(target_os = "macos") {
        "install Xcode's command-line tools: xcode-select --install"
    } else {
        "install a C compiler: sudo apt install build-essential (Debian, Ubuntu) or sudo dnf install gcc (Fedora)"
    };
    format!("can't find the linker `{name}`, which building a program needs; {install}")
}

#[cfg(windows)]
fn linker() -> Result<Command, String> {
    cc::windows_registry::find_tool("x86_64-pc-windows-msvc", "link.exe")
        .map(|tool| tool.to_command())
        .ok_or_else(|| {
            "can't find the MSVC linker `link.exe`; install the Visual Studio Build Tools with the C++ workload".to_string()
        })
}

#[cfg(not(windows))]
fn linker() -> Result<Command, String> {
    Ok(Command::new(
        std::env::var("CC").unwrap_or_else(|_| "cc".into()),
    ))
}

#[cfg(test)]
mod tests {
    /// A missing linker is named, with what to install (decision 0080).
    #[test]
    fn a_missing_linker_says_what_to_install() {
        let cmd = std::process::Command::new("kavach-no-such-linker");
        let message = super::run(cmd).expect_err("there is no such linker");
        assert!(message.contains("`kavach-no-such-linker`"), "{message}");
        assert!(message.contains("install"), "{message}");
    }

    /// A build stopped half way leaves its folder; the next one removes
    /// it once it is old, and only the folders of its own kind.
    #[test]
    fn old_build_folders_are_removed() {
        let dir = std::env::temp_dir().join(format!("kavach-link-test-{}", std::process::id()));
        for name in [
            "kavach-build-123",
            "kavach-build-lib-9",
            "kavach-build-notes",
        ] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
        }
        super::remove_old(&dir, "kavach-build", std::time::Duration::ZERO);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(left, ["kavach-build-lib-9", "kavach-build-notes"]);
        super::remove_old(&dir, "kavach-build", std::time::Duration::from_secs(86_400));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
