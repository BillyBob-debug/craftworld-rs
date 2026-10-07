# Daily hands-off loop (docs/workflow.md section 7).
# Step 1: local Integrator via `claude -p` (plan limits).
# Step 2: cloud batch driver via `claude --cloud` (cloud credit).
# Run by the Task Scheduler task LBP2-DailyBatch; safe to run by hand.
$ErrorActionPreference = 'Continue'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
$claude = Join-Path $env:USERPROFILE '.local\bin\claude.exe'

$logDir = Join-Path $repo 'local-analysis\daily-batch'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$stamp = Get-Date -Format 'yyyy-MM-dd'
$log = Join-Path $logDir "$stamp.log"
$n = 1
while (Test-Path -LiteralPath $log) {
    $n++
    $log = Join-Path $logDir "$stamp-$n.log"
}
function Write-Log([string]$text) {
    "[$(Get-Date -Format 'HH:mm:ss')] $text" | Out-File -FilePath $log -Append -Encoding utf8
}

if (-not (Test-Path -LiteralPath $claude)) {
    Write-Log "claude.exe not found at $claude; aborting."
    exit 1
}
. (Join-Path $repo 'scripts\use-local-rust.ps1')

git fetch -q origin 2>&1 | Out-File -FilePath $log -Append -Encoding utf8
$headBefore = (git rev-parse HEAD).Trim()
$pending = @(git branch -r --no-merged origin/main 2>$null | ForEach-Object { $_.Trim() } |
    Where-Object { $_ -like 'origin/cloud/*' -or $_ -like 'origin/claude/*' })
$firstRun = -not (Test-Path -LiteralPath (Join-Path $repo 'docs\backlog.md'))
Write-Log "HEAD $headBefore; unmerged cloud branches: $($pending.Count); first run: $firstRun"

$branch = (git rev-parse --abbrev-ref HEAD).Trim()
$dirtyStart = (git status --porcelain | Measure-Object).Count
$step1Exit = 0
if ($branch -ne 'main' -or $dirtyStart -gt 0) {
    # Another session may be working in this checkout; never stash or reset its work.
    Write-Log "Step 1 skipped: checkout busy (branch $branch, $dirtyStart changed paths)."
} elseif ($pending.Count -gt 0 -or $firstRun) {
    $prompt = @"
Role: Integrator, per docs/workflow.md sections 6 and 7 step 1. You run unattended; do not ask questions.
Follow AGENTS.md and CLAUDE.md (context budget, capped output). The Rust toolchain is already on PATH.
1. git checkout main; git pull --ff-only origin main.
2. For each remote branch from ``git branch -r --no-merged origin/main`` matching origin/cloud/* or origin/claude/*, in D-number order
   (from the branch name or its PR title):
   check it out, run cargo fmt --all --check, the capped cargo test and clippy for the crates it touches,
   python -m unittest discover -s tests, and the PC verify command from its PR body if one exists.
   Pass: merge it into main (no fast-forward). Fail: note at most 10 lines of output and skip it.
3. Overwrite docs/batch-report.md: merged branches, failed branches with their output, new counts.
4. If docs/backlog.md is missing or stale, refresh it from the latest census (ranked, counts only, no asset bytes).
5. If anything merged: one batch STATUS.md entry (max 40 lines), MODLOG.md (max 6 lines), overwrite STATUS-CURRENT.md.
6. Commit and git push origin main. Never force-push, never commit game data or local-analysis output.
If no unmerged cloud branches exist, only do steps 3-4 and 6.
"@
    $allowed = @(
        'Read', 'Edit', 'Write', 'Glob', 'Grep',
        'Bash(git:*)', 'Bash(cargo:*)', 'Bash(python:*)', 'Bash(tail:*)', 'Bash(./scripts/run-*)',
        'PowerShell(git *)', 'PowerShell(cargo *)', 'PowerShell(python *)',
        'PowerShell(./scripts/run-*)', 'PowerShell(. ./scripts/use-local-rust.ps1*)'
    )
    Write-Log 'Step 1: Integrator starting.'
    & $claude -p $prompt --allowedTools $allowed --output-format text 2>&1 |
        Out-File -FilePath $log -Append -Encoding utf8
    $step1Exit = $LASTEXITCODE
    Write-Log "Step 1 exit code: $step1Exit"
} else {
    Write-Log 'Step 1 skipped: nothing to integrate.'
}

if ($step1Exit -ne 0) {
    $headAfter = (git rev-parse HEAD).Trim()
    $dirty = (git status --porcelain | Measure-Object).Count
    if ($headAfter -ne $headBefore -or $dirty -gt 0) {
        Write-Log "Step 1 failed and modified main (HEAD $headAfter, $dirty dirty paths); step 2 not launched."
        exit $step1Exit
    }
}

Write-Log 'Step 2: launching cloud batch driver.'
# --cloud needs an interactive terminal, so it gets its own console window.
$driver = 'Role: Batch driver, per docs/workflow.md section 7 step 2. Repo chilladept/lbp2-asset-explorer. ' +
    'Skip any task on two-texture x COLOR materials; that route is being done on the PC. ' +
    'Name branches cloud/D<NN>-<slug> if allowed; otherwise put the D-number in the PR title.'
$proc = Start-Process -FilePath $claude -ArgumentList @('--cloud', "`"$driver`"") -WorkingDirectory $repo `
    -WindowStyle Minimized -PassThru
if (-not $proc.WaitForExit(600000)) {
    Write-Log 'Step 2: --cloud client still open after 10 minutes; closing it (the cloud session keeps running).'
    $proc.Kill()
}
Write-Log "Step 2 exit code: $($proc.ExitCode)"
