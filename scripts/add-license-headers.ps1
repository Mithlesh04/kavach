# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

# Adds the license header (decision 0133) to every source file that lacks
# one. The files are the ones scripts/check-repo.ps1 reports, so the scope
# and what is left out live in one place. A file without a header is new,
# so its header gets this year. A header already there is never changed.
# Usage: powershell -File scripts/add-license-headers.ps1 [file...]
#   With files named, adds the header to just those.
# Twin: scripts/add-license-headers.sh (keep both in sync)

$year = (Get-Date).Year
$utf8 = New-Object System.Text.UTF8Encoding $false

# Writes the header at the top of the file, after a shebang (`#!`, but not
# Rust's `#![`) if it has one, with a blank line before the rest. Line
# endings stay LF.
function Add-Header($path) {
    $ErrorActionPreference = 'Stop'
    $name = Split-Path -Leaf $path
    if ($path -match '\.(rs|kav|mjs|ts|js)$') { $c = '//' }
    elseif ($path -match '\.(sh|ps1)$') { $c = '#' }
    elseif ($path -match 'hooks[/\\]' -and $name -notmatch '\.') { $c = '#' }  # git hooks have no extension
    else { Write-Output "skipped   ${path}: not a file type that carries the header"; return }
    $full = (Resolve-Path -LiteralPath $path).Path
    $text = [IO.File]::ReadAllText($full, $utf8)
    $head = @($text -split "`n" | Select-Object -First 5)
    if (($head -cmatch 'Copyright \(c\) [0-9]{4} Mithlesh and the Kavach contributors\s*$') -and
        ($head -cmatch 'SPDX-License-Identifier: Apache-2\.0 OR MIT\s*$')) { return }
    $shebang = ''
    if ($text.StartsWith('#!') -and -not $text.StartsWith('#![')) {
        $end = $text.IndexOf("`n")
        if ($end -lt 0) { $shebang = $text + "`n"; $text = '' }
        else { $shebang = $text.Substring(0, $end + 1); $text = $text.Substring($end + 1) }
    }
    $header = "$c Copyright (c) $year Mithlesh and the Kavach contributors`n" +
        "$c SPDX-License-Identifier: Apache-2.0 OR MIT`n" +
        "$c https://kavachlang.com | https://github.com/Mithlesh04/kavach`n"
    if ($text -ne '' -and -not $text.StartsWith("`n")) { $header += "`n" }
    [IO.File]::WriteAllText($full, $shebang + $header + $text, $utf8)
    Write-Output "added     $path"
}

if ($args.Count -gt 0) {
    foreach ($f in $args) { Add-Header $f }
    exit 0
}
Set-Location (Split-Path -Parent $PSScriptRoot)
$report = & "$PSScriptRoot\check-repo.ps1"
foreach ($line in $report) {
    if ($line -match '^NO HEADER \./(.*): no license header in its first 5 lines\.') { Add-Header $Matches[1] }
}
