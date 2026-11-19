# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

# Checks the repository rules from AGENTS.md:
#   1. no file we write is longer than 500 lines
#   2. every relative Markdown link points to a file (and heading) that exists
#   3. every source file starts with the license header (decision 0133)
# Usage: powershell -File scripts/check-repo.ps1     Twin: scripts/check-repo.sh (keep both in sync)

$MaxLines = 500
$root = Split-Path -Parent $PSScriptRoot
$fail = $false

# The files git keeps or would keep: what .gitignore leaves out (build
# output, examples' audit logs) is not ours to check. Generated files
# (benchmark runs' results among them) are not counted either, nor
# third-party code we vendor (`vendor/` folders).
# Keep this list in sync with the .sh twin.
$files = git -C $root ls-files --cached --others --exclude-standard |
    Where-Object { $_ -notmatch '\.lock$' -and $_ -notmatch 'tests/found/' -and $_ -notmatch 'package-lock\.json$' -and
        $_ -notmatch 'website/data/benchmarks\.json$' -and $_ -notmatch '/vendor/' -and
        $_ -notmatch 'benchmarks/[^/]+/results/' } |
    ForEach-Object { Join-Path $root $_ } |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    ForEach-Object { Get-Item -LiteralPath $_ -Force }

function Get-Rel($path) { '.' + $path.Substring($root.Length).Replace('\', '/') }

# Lines outside ``` code blocks.
function Get-ProseLines($path) {
    $fence = $false
    foreach ($line in [IO.File]::ReadAllLines($path)) {
        if ($line -match '^```') { $fence = -not $fence; continue }
        if (-not $fence) { $line }
    }
}

# GitHub anchor for a heading: lowercase, punctuation removed, spaces become hyphens.
function Get-Slugs($path) {
    Get-ProseLines $path | Where-Object { $_ -match '^#+ ' } | ForEach-Object {
        (($_ -replace '^#+ +', '').ToLower() -replace '[^\w\s-]', '' -replace ' ', '-')
    }
}

# 1. File length
foreach ($f in $files) {
    if ($f.Length -eq 0) { continue }
    $bytes = [IO.File]::ReadAllBytes($f.FullName)
    if ([Array]::IndexOf($bytes[0..([Math]::Min($bytes.Length, 8000) - 1)], [byte]0) -ge 0) { continue }  # binary
    $n = [IO.File]::ReadAllLines($f.FullName).Length
    if ($n -gt $MaxLines) {
        Write-Output "TOO LONG  $(Get-Rel $f.FullName): $n lines (limit $MaxLines). Split it by responsibility."
        $fail = $true
    }
}

# 2. Relative links in Markdown, outside code blocks
foreach ($f in $files | Where-Object { $_.Extension -eq '.md' }) {
    $text = (Get-ProseLines $f.FullName) -join "`n"
    foreach ($m in [regex]::Matches($text, '\]\(([^) ]+)\)')) {
        $link = $m.Groups[1].Value
        if ($link -match '^(https?|mailto):') { continue }
        $path, $anchor = $link -split '#', 2
        if ($path) { $target = Join-Path $f.DirectoryName $path } else { $target = $f.FullName }
        if (-not (Test-Path $target)) {
            Write-Output "BROKEN    $(Get-Rel $f.FullName) -> $link (no such file)"
            $fail = $true
        }
        elseif ($anchor -and (Test-Path $target -PathType Leaf) -and ((Get-Slugs $target) -notcontains $anchor)) {
            Write-Output "BROKEN    $(Get-Rel $f.FullName) -> $link (no such heading)"
            $fail = $true
        }
    }
}

# 3. License header: within its first 5 lines, each source file in scope
# names the copyright holders and the SPDX license (decision 0133). A file
# marked `@generated` or "do not edit" there is skipped. Keep the scope and
# the skip list in sync with the .sh twin.
$HeaderScope = '^\./(compiler|runtime|tools|stdlib|interop|scripts|editors)/.*\.(rs|kav|sh|ps1|mjs|ts|js)$|^\./scripts/hooks/[^/.]+$'
# Left out: test fixtures whose text or line numbers a test compares.
$HeaderSkip = @(
    '^\./editors/vscode/tests/[^/]*\.kav$'
)
foreach ($f in $files) {
    $rel = Get-Rel $f.FullName
    if ($rel -cnotmatch $HeaderScope) { continue }
    if ($HeaderSkip | Where-Object { $rel -cmatch $_ }) { continue }
    $reader = [IO.File]::OpenText($f.FullName)
    $head = @(for ($i = 0; $i -lt 5 -and -not $reader.EndOfStream; $i++) { $reader.ReadLine() })
    $reader.Dispose()
    $holder = $head -cmatch 'Copyright \(c\) [0-9]{4} Mithlesh and the Kavach contributors\s*$'
    $spdx = $head -cmatch 'SPDX-License-Identifier: Apache-2\.0 OR MIT\s*$'
    $made = $head -match '@generated|do not edit'
    if (-not (($holder -and $spdx) -or $made)) {
        Write-Output "NO HEADER ${rel}: no license header in its first 5 lines. Add it with scripts/add-license-headers.sh or .ps1."
        $fail = $true
    }
}

if ($fail) { exit 1 }
Write-Output 'check-repo: all good'
exit 0
