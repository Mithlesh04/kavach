# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

# Installs Kavach on Windows (decision 0080): downloads the newest
# release archive (or takes one with `-From <file>`), unpacks it to
# %LOCALAPPDATA%\Kavach, and puts its bin\ on the user's PATH. Then
# checks for the MSVC linker, which building programs needs.
param([string]$From = "")
$ErrorActionPreference = "Stop"

$repo = "Mithlesh04/kavach"
$home_dir = if ($env:KAVACH_HOME) { $env:KAVACH_HOME } else { Join-Path $env:LOCALAPPDATA "Kavach" }

$archive = $From
$temp = ""
if (-not $archive) {
    Write-Host "finding the newest release of $repo for windows-x86_64"
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $asset = $release.assets | Where-Object { $_.name -like "*-windows-x86_64.zip" } | Select-Object -First 1
    if (-not $asset) {
        Write-Error "no release archive for windows-x86_64 yet. Build from source instead: https://github.com/$repo#readme"
    }
    $temp = Join-Path $env:TEMP $asset.name
    Write-Host "downloading $($asset.browser_download_url)"
    Invoke-WebRequest $asset.browser_download_url -OutFile $temp
    $archive = $temp
}

Write-Host "unpacking into $home_dir"
$new = "$home_dir.new"
if (Test-Path $new) { Remove-Item -Recurse -Force $new }
Expand-Archive -Path $archive -DestinationPath $new
# The archive holds one folder; its contents become the install.
$inner = Get-ChildItem $new | Select-Object -First 1
if (Test-Path $home_dir) { Remove-Item -Recurse -Force $home_dir }
Move-Item $inner.FullName $home_dir
Remove-Item -Recurse -Force $new
if ($temp) { Remove-Item -Force $temp }

$bin = Join-Path $home_dir "bin"
# Kavach's folder goes first in the user's PATH, so an older kavach found
# elsewhere doesn't win, and into this window's PATH too, so `kavach`
# works at once when the script runs here (`irm … | iex`).
# KAVACH_NO_MODIFY_PATH=1 leaves the PATH alone.
if (-not $env:KAVACH_NO_MODIFY_PATH) {
    $user_path = [Environment]::GetEnvironmentVariable("Path", "User")
    $others = ($user_path -split ";") | Where-Object { $_ -and $_ -ne $bin }
    [Environment]::SetEnvironmentVariable("Path", (@($bin) + $others) -join ";", "User")
    $env:Path = (@($bin) + (($env:Path -split ";") | Where-Object { $_ -and $_ -ne $bin })) -join ";"
    Write-Host "added $bin to your PATH; other open terminals and editors see it once restarted"
    $found = Get-Command kavach -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found -and $found.Source -ne (Join-Path $bin "kavach.exe")) {
        Write-Host "note: another kavach comes first: $($found.Source); remove or rename it"
    }
}
Write-Host "installed $(& (Join-Path $bin 'kavach.exe') --version)"

$msvc = Get-ChildItem "${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio" `
    -Recurse -Filter link.exe -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $msvc) {
    Write-Host "note: building programs needs the Visual Studio Build Tools (C++ workload):"
    Write-Host "      https://visualstudio.microsoft.com/visual-cpp-build-tools/"
}
