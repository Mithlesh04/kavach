# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

# Runs scripts/run-benchmarks.sh in WSL (the benchmarks compare with g++).
# Usage: powershell -File scripts/run-benchmarks.ps1 [kavach] [benchmarks/math]
$root = (Resolve-Path "$PSScriptRoot\..").Path
$drive = $root.Substring(0, 1).ToLower()
$linux = "/mnt/$drive" + $root.Substring(2).Replace('\', '/')
$extra = ($args | ForEach-Object { "'" + $_ + "'" }) -join " "
wsl.exe bash -lc "cd '$linux' && sh scripts/run-benchmarks.sh $extra"
