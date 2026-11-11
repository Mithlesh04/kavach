# Packages the speed kit for Windows (to-do: "A speed comparison anyone
# can run"): every benchmark that has a C++ twin and runs alone, built in
# Kavach (`kavach build`) and in C++ (MSVC `cl /O2`, with the C runtime
# linked in so it runs on a fresh machine), with their source, and
# `compare.exe`, which times them. Writes .kavach\dist\kavach-speed-
# <version>-windows-x86_64.zip and a .sha256. Needs cargo, LLVM's tools
# for Kavach (decision 0018) and the Visual Studio Build Tools.
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
$repo = Get-Location

$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "target" }
Write-Host "building kavach and compare (release)"
cargo build -q --release -p kavach-cli -p kavach-speedkit
if ($LASTEXITCODE -ne 0) { exit 1 }
$kavach = Resolve-Path "$target\release\kavach.exe"

# MSVC's compiler, through Visual Studio's own setup of its environment.
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { Write-Error "the Visual Studio Build Tools (C++ workload) are needed" }
& (Join-Path $vs "Common7\Tools\Launch-VsDevShell.ps1") -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null
Set-Location $repo
# `cl` writes its banner to stderr, which PowerShell would take for an error.
$cl_version = (cmd /c "cl 2>&1" | Select-Object -First 1).Trim()

$commit = (& git rev-parse --short HEAD).Trim()
$version = "0.1.0-$commit"
$name = "kavach-speed-$version-windows-x86_64"
$kit = Join-Path $repo ".kavach\dist\$name"
if (Test-Path $kit) { Remove-Item -Recurse -Force $kit }
foreach ($d in "programs\kavach", "programs\cpp") {
    New-Item -ItemType Directory -Force (Join-Path $kit $d) | Out-Null
}
Copy-Item "$target\release\compare.exe" $kit
$objects = Join-Path $kit "obj"
New-Item -ItemType Directory -Force $objects | Out-Null

$kavach_version = (& $kavach --version).Trim() -replace '^kavach ', ''
$lines = @(
    "# Kavach $kavach_version, built with kavach build",
    "# C++: $cl_version, cl /O2 /MT, 8 MB of stack as on Linux"
)
# Suites whose programs run alone, with no GPU, network or arguments.
foreach ($suite in "loops", "everyday", "chains", "math", "lookups", "bytes", "channels", "tasks") {
    New-Item -ItemType Directory -Force (Join-Path $kit "source\kavach\$suite"), (Join-Path $kit "source\cpp\$suite") | Out-Null
    foreach ($src in Get-ChildItem "benchmarks\$suite\kavach\*.kav") {
        $bench = $src.BaseName
        $cpp = "benchmarks\$suite\cpp\$bench.cpp"
        if (-not (Test-Path $cpp)) { continue }
        Write-Host "  $suite/$bench"
        & $kavach build $src.FullName -o (Join-Path $kit "programs\kavach\$bench.exe") | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Error "kavach build failed for $suite/$bench" }
        # Linux's 8 MB of stack, not Windows' 1 MB: `stacks` recurses deeply.
        & cl /nologo /O2 /MT /EHsc /std:c++20 /Fo"$objects\\" /Fe"$(Join-Path $kit "programs\cpp\$bench.exe")" $cpp /link /STACK:8388608 | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Error "cl failed for $suite/$bench" }
        Copy-Item $src.FullName (Join-Path $kit "source\kavach\$suite")
        Copy-Item $cpp (Join-Path $kit "source\cpp\$suite")
        $lines += "$suite $bench"
    }
}
Remove-Item -Recurse -Force $objects
$lines | Out-File -Encoding ascii (Join-Path $kit "kit.txt")

@"
Kavach speed kit $version (Windows, x86_64)

Run compare.exe in a terminal (PowerShell or Command Prompt). It checks
that each Kavach program prints the same result as its C++ twin, then
runs the two in turns and prints the best time of each and how many times
faster Kavach is. The table is also saved in results.txt. Options:
--runs N (default 5), --only <name>.

Nothing else is needed: the programs are already built. Their source is in
source\kavach and source\cpp; the same files, with the measured results,
are at https://github.com/Mithlesh04/kavach/tree/main/benchmarks

The C++ here is built by Microsoft's compiler (MSVC), as Windows builds it.
The results published in the repository compare against g++ on Linux,
whose library is faster at some things (hash maps, for one), so there
Kavach's lead is smaller on those benchmarks.

Close other busy programs first: they slow both languages, but not always
alike. kit.txt says what built the programs.
"@ | Out-File -Encoding ascii (Join-Path $kit "README.txt")
Copy-Item LICENSE-APACHE, LICENSE-MIT $kit

$zip = Join-Path $repo ".kavach\dist\$name.zip"
Compress-Archive -Path $kit -DestinationPath $zip -Force
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
"$hash  $name.zip" | Out-File -Encoding ascii "$zip.sha256"
Remove-Item -Recurse -Force $kit
Write-Host "wrote .kavach\dist\$name.zip"
