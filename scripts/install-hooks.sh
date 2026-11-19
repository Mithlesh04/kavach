#!/bin/sh
# Copyright (c) 2026 Mithlesh and the Kavach contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
# https://kavachlang.com | https://github.com/Mithlesh04/kavach

# Points git at the versioned hooks in scripts/hooks (rules: AGENTS.md section 10).
# Usage: sh scripts/install-hooks.sh     Twin: scripts/install-hooks.ps1
cd "$(dirname "$0")/.." || exit 2
git rev-parse --git-dir >/dev/null 2>&1 || { echo "install-hooks: not a git repository. Run 'git init' first." >&2; exit 1; }
git config core.hooksPath scripts/hooks
chmod +x scripts/hooks/* 2>/dev/null
git update-index --chmod=+x scripts/hooks/* 2>/dev/null   # only works once the hooks are committed
echo "install-hooks: done. Commit messages are now checked by scripts/hooks/commit-msg."
