# List (default) or delete cargo build trees that no live worktree needs.
# A tree is a candidate when it is a target* folder of the canonical checkout or of a git worktree, or
# an E:\Projects-style sibling folder named *target* (an old CARGO_TARGET_DIR), and:
#   - its worktree is gone, or its worktree's branch is merged into origin/main (or HEAD is detached
#     on a commit main already contains), and
#   - nothing was built in it for -MinIdleHours (default 12).
# The canonical checkout's own target/ and every worktree named in -Keep are never candidates.
# Usage: scripts/prune-targets.ps1 [-Keep wt-a,wt-b] [-MinIdleHours 12] [-Delete]
param([string[]]$Keep = @(), [double]$MinIdleHours = 12, [switch]$Delete)
$ErrorActionPreference = 'Stop'
$repo = (git -C $PSScriptRoot rev-parse --path-format=absolute --git-common-dir) | Split-Path -Parent
$parent = Split-Path -Parent $repo
git -C $repo fetch -q origin main 2>$null

function Last-Build([string]$dir) {
    $marks = Get-ChildItem -LiteralPath $dir -Directory -ErrorAction SilentlyContinue |
        ForEach-Object { Get-Item -LiteralPath (Join-Path $_.FullName '.fingerprint') -ErrorAction SilentlyContinue }
    $latest = $marks | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($latest) { $latest.LastWriteTime } else { (Get-Item -LiteralPath $dir).LastWriteTime }
}
function Size-GB([string]$dir) {
    $bytes = (Get-ChildItem -LiteralPath $dir -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
    [math]::Round($bytes / 1GB, 1)
}

# Worktrees: path -> merged?
$worktrees = @{}
$entry = @{}
foreach ($line in (git -C $repo worktree list --porcelain) + '') {
    if ($line -like 'worktree *') { $entry = @{ path = (Resolve-Path -LiteralPath $line.Substring(9) -ErrorAction SilentlyContinue).Path } }
    elseif ($line -like 'HEAD *') { $entry.head = $line.Substring(5) }
    elseif ($line -like 'branch *') { $entry.branch = $line.Substring(7) }
    elseif ($line -eq '' -and $entry.path) {
        git -C $repo merge-base --is-ancestor $entry.head origin/main 2>$null
        $worktrees[$entry.path] = @{ merged = ($LASTEXITCODE -eq 0); name = $(if ($entry.branch) { $entry.branch -replace '^refs/heads/', '' } else { 'detached' }) }
        $entry = @{}
    }
}

$rows = @()
foreach ($wt in $worktrees.Keys) {
    foreach ($t in Get-ChildItem -LiteralPath $wt -Directory -Filter 'target*' -ErrorAction SilentlyContinue) {
        $isCanonicalTarget = ($wt -eq $repo -and $t.Name -eq 'target')
        $kept = $isCanonicalTarget -or ($Keep -contains (Split-Path -Leaf $wt))
        $why = if ($kept) { 'kept' } elseif ($wt -eq $repo) { 'extra target in canonical checkout' } elseif ($worktrees[$wt].merged) { "branch $($worktrees[$wt].name) merged" } else { $null }
        $rows += [pscustomobject]@{ Path = $t.FullName; Owner = "$(Split-Path -Leaf $wt) ($($worktrees[$wt].name))"; Why = $why; Kept = $kept }
    }
}
foreach ($t in Get-ChildItem -LiteralPath $parent -Directory | Where-Object { $_.Name -like '*target*' }) {
    $rows += [pscustomobject]@{ Path = $t.FullName; Owner = 'none (old CARGO_TARGET_DIR)'; Why = 'no worktree'; Kept = $false }
}

$now = Get-Date
$total = 0
foreach ($r in $rows) {
    $idle = ($now - (Last-Build $r.Path)).TotalHours
    $candidate = (-not $r.Kept) -and $r.Why -and ($idle -ge $MinIdleHours)
    $size = Size-GB $r.Path
    $state = if ($candidate) { 'PRUNE' } elseif ($r.Kept) { 'keep ' } else { 'live ' }
    '{0} {1,7:N1} GB  idle {2,6:N0} h  {3}  [{4}; {5}]' -f $state, $size, $idle, $r.Path, $r.Owner, $(if ($r.Why) { $r.Why } else { 'unmerged branch' })
    if ($candidate) {
        $total += $size
        if ($Delete) { Remove-Item -LiteralPath $r.Path -Recurse -Force }
    }
}
'{0} {1:N1} GB in PRUNE rows' -f $(if ($Delete) { 'deleted' } else { 'would free' }), $total
