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
$user_path = [Environment]::GetEnvironmentVariable("Path", "User")
if (($user_path -split ";") -notcontains $bin) {
    [Environment]::SetEnvironmentVariable("Path", "$user_path;$bin", "User")
    Write-Host "added $bin to your PATH (new terminals see it)"
}
Write-Host "installed $(& (Join-Path $bin 'kavach.exe') --version)"

$msvc = Get-ChildItem "${env:ProgramFiles}\Microsoft Visual Studio", "${env:ProgramFiles(x86)}\Microsoft Visual Studio" `
    -Recurse -Filter link.exe -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $msvc) {
    Write-Host "note: building programs needs the Visual Studio Build Tools (C++ workload):"
    Write-Host "      https://visualstudio.microsoft.com/visual-cpp-build-tools/"
}
