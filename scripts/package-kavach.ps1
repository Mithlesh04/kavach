# Packages an installable Kavach (decision 0080) for Windows: a zip with
# bin\ (kavach.exe, kv.exe), lib\ (the runtime library), llvm\ (opt and
# llc from the Rust toolchain) and the licences, plus a .sha256 beside
# it, in .kavach\dist\. Needs cargo and rustup's llvm-tools component.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
Write-Host "building the compiler (release)"
cargo build -q --release -p kavach-cli
if ($LASTEXITCODE -ne 0) { exit 1 }

$sysroot = (& rustc --print sysroot).Trim()
$opt = Get-ChildItem -Path $sysroot -Recurse -Filter opt.exe | Select-Object -First 1
if (-not $opt) {
    Write-Error "opt.exe not found; run: rustup component add llvm-tools"
}
$tools = $opt.DirectoryName
if (-not (Test-Path (Join-Path $tools "llc.exe"))) {
    Write-Error "llc.exe not found next to opt.exe"
}

$commit = (& git rev-parse --short HEAD).Trim()
$version = "0.1.0-$commit"
$name = "kavach-$version-windows-x86_64"
$dist = ".kavach\dist\$name"
if (Test-Path $dist) { Remove-Item -Recurse -Force $dist }
foreach ($d in "bin", "lib", "llvm") {
    New-Item -ItemType Directory -Force (Join-Path $dist $d) | Out-Null
}

Copy-Item "$target\release\kavach.exe" "$dist\bin\kavach.exe"
Copy-Item "$target\release\kavach.exe" "$dist\bin\kv.exe"
# The runtime library sits in deps\ with a hash; the newest is current.
$lib = Join-Path "$target\release" "kavach_runtime_native.lib"
if (-not (Test-Path $lib)) {
    $lib = (Get-ChildItem "$target\release\deps\kavach_runtime_native-*.lib" |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
}
Copy-Item $lib "$dist\lib\kavach_runtime_native.lib"
Copy-Item (Join-Path $tools "opt.exe") "$dist\llvm\"
Copy-Item (Join-Path $tools "llc.exe") "$dist\llvm\"
Copy-Item LICENSE-APACHE, LICENSE-MIT $dist
@"
opt and llc come from the Rust toolchain's llvm-tools component and are
licensed under Apache-2.0 WITH LLVM-exception: https://llvm.org/LICENSE.txt
"@ | Out-File -Encoding utf8 "$dist\llvm\NOTICE.txt"
@"
Kavach $version

Add bin\ to your PATH, or run scripts\install-kavach.ps1 from the
repository. Building programs also needs the Visual Studio Build Tools
(C++ workload) for linking; kavach says so if they are missing.
Docs: https://mithlesh04.github.io/kavach/
"@ | Out-File -Encoding utf8 "$dist\README.txt"

Compress-Archive -Path $dist -DestinationPath ".kavach\dist\$name.zip" -Force
$hash = (Get-FileHash ".kavach\dist\$name.zip" -Algorithm SHA256).Hash.ToLower()
"$hash  $name.zip" | Out-File -Encoding utf8 ".kavach\dist\$name.zip.sha256"
Remove-Item -Recurse -Force $dist
Write-Host "wrote .kavach\dist\$name.zip"
