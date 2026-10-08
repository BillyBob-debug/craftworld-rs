param(
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string]$SampleManifest,
    # Data root (references/, local-analysis/toolchain, game-data/). Defaults to the checkout holding this script.
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)
# D119: Toolkit RPlan Thing-array oracle. Output must be a new JSON under ./local-analysis
# of the current directory (the oracle refuses anything else).
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $ProjectRoot).Path
$toolkit = Join-Path $root 'references/toolkit'
if ((& git -C $toolkit rev-parse HEAD).Trim() -ne 'd6a2ef80b5a09999f08f23a803b516096055e5c9') { throw 'Toolkit pin differs' }
if (& git -C $toolkit status --porcelain --untracked-files=no) { throw 'Modified Toolkit source' }
$javaRoot = (Get-ChildItem -LiteralPath (Join-Path $root 'local-analysis/toolchain/java') -Directory | Select-Object -First 1).FullName
$jar = Join-Path $toolkit 'lib/cwlib/target/cwlib-0.1.jar'
$classes = Join-Path (Get-Location) 'local-analysis/oracle/plan-thing-classes'
New-Item -ItemType Directory -Force -Path $classes | Out-Null
& "$javaRoot/bin/javac.exe" -cp $jar -d $classes (Join-Path $PSScriptRoot 'PlanThingOracle.java')
if ($LASTEXITCODE -ne 0) { throw 'Plan Thing oracle compilation failed' }
$archives = @((Join-Path $root 'game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC')) +
    @(Get-ChildItem -LiteralPath (Join-Path $root 'game-data/a1-update-0133/USRDIR/patches') -Filter 'patch_*.farc' | Sort-Object Name | ForEach-Object { $_.FullName })
& "$javaRoot/bin/java.exe" '-Xmx1g' '-Djava.awt.headless=true' -cp "$classes;$jar" PlanThingOracle `
    $SampleManifest $Output @archives
if ($LASTEXITCODE -ne 0) { throw 'Plan Thing oracle failed' }
