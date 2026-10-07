# Clickable launcher for this development checkout. No game input is modified.
$ErrorActionPreference = 'Stop'
try {
    $viewerRoot = Split-Path -Parent $PSScriptRoot
    $viewerExe = Join-Path $viewerRoot 'target/debug/lbp-bevy-viewer.exe'
    $viewerSnapshot = Join-Path $viewerRoot 'local-analysis/a2-inventory-verified.json'
    $viewerGame = Join-Path $viewerRoot 'game-data/a1-disc'
    foreach ($viewerInput in @($viewerExe, $viewerSnapshot, $viewerGame)) {
        if (-not (Test-Path -LiteralPath $viewerInput)) {
            throw "Required local file/directory missing: $viewerInput. See docs/bevy-preview.md for setup/build instructions."
        }
    }
    . (Join-Path $PSScriptRoot 'use-local-rust.ps1')
    Start-Process -FilePath $viewerExe -WorkingDirectory $viewerRoot -WindowStyle Hidden -ArgumentList @(
        '--snapshot', 'local-analysis/a2-inventory-verified.json',
        '--game-dir', 'game-data/a1-disc', '--select-guid', '00010e8d'
    ) | Out-Null
    exit 0
} catch {
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
