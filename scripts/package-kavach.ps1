# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

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
# The version is the workspace's, in Cargo.toml; the commit tells builds apart.
$base = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$version = "$base-$commit"
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
Docs: https://kavachlang.com/
"@ | Out-File -Encoding utf8 "$dist\README.txt"

# Each file is added under a name with `/` between folders, which every
# unzip reads; Windows PowerShell 5.1's Compress-Archive writes `\`.
Add-Type -AssemblyName System.IO.Compression.FileSystem
function Write-Zip($from, $to) {
    $root = (Resolve-Path $from).Path
    $top = Split-Path $root -Leaf
    if (Test-Path $to) { Remove-Item -Force $to }
    $zip = [IO.Compression.ZipFile]::Open($to, "Create")
    foreach ($f in Get-ChildItem -Recurse -File $root) {
        $inside = $f.FullName.Substring($root.Length + 1).Replace("\", "/")
        [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $f.FullName, "$top/$inside", "Optimal")
    }
    $zip.Dispose()
}
$zip = Join-Path (Get-Location) ".kavach\dist\$name.zip"
Write-Zip $dist $zip
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
# No byte order mark and a plain newline, so `sha256sum -c` reads it.
[IO.File]::WriteAllText("$zip.sha256", "$hash  $name.zip`n")
Remove-Item -Recurse -Force $dist
Write-Host "wrote .kavach\dist\$name.zip"
