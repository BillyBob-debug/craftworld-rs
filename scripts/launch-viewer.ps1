# Clickable launcher for this development checkout. No game input is modified.
$ErrorActionPreference = 'Stop'
try {
    $viewerRoot = Split-Path -Parent $PSScriptRoot
    $viewerExe = Join-Path $viewerRoot 'target/debug/lbp-bevy-viewer.exe'
    # Prefer disc + installed update 01.33 (docs/update-overlay.md); fall back to the disc alone.
    $snapshotArg = 'local-analysis/d68-inventory-0133-verified.json'
    $gameArg = 'game-data/a1-disc;game-data/a1-update-0133'
    if (-not ((Test-Path -LiteralPath (Join-Path $viewerRoot $snapshotArg)) -and
              (Test-Path -LiteralPath (Join-Path $viewerRoot 'game-data/a1-update-0133')))) {
        $snapshotArg = 'local-analysis/a2-inventory-verified.json'
        $gameArg = 'game-data/a1-disc'
    }
    $viewerSnapshot = Join-Path $viewerRoot $snapshotArg
    $viewerGame = Join-Path $viewerRoot 'game-data/a1-disc'
    foreach ($viewerInput in @($viewerExe, $viewerSnapshot, $viewerGame)) {
        if (-not (Test-Path -LiteralPath $viewerInput)) {
            throw "Required local file/directory missing: $viewerInput. See docs/bevy-preview.md for setup/build instructions."
        }
    }
    . (Join-Path $PSScriptRoot 'use-local-rust.ps1')
    Start-Process -FilePath $viewerExe -WorkingDirectory $viewerRoot -WindowStyle Hidden -ArgumentList @(
        '--snapshot', $snapshotArg,
        '--game-dir', "`"$gameArg`"", '--select-guid', '00010e8d'
    ) | Out-Null
    exit 0
} catch {
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
