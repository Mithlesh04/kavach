# Points git at the versioned hooks in scripts/hooks (rules: AGENTS.md section 10).
# Usage: powershell -File scripts/install-hooks.ps1     Twin: scripts/install-hooks.sh
# The hooks themselves are sh scripts; Git for Windows runs them with its own sh.
Set-Location (Split-Path -Parent $PSScriptRoot)
git rev-parse --git-dir *> $null
if ($LASTEXITCODE -ne 0) { Write-Error "install-hooks: not a git repository. Run 'git init' first."; exit 1 }
git config core.hooksPath scripts/hooks
Write-Output 'install-hooks: done. Commit messages are now checked by scripts/hooks/commit-msg.'
