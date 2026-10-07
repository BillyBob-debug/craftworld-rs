param(
    [Parameter(Mandatory=$true)][string]$Output,
    [string]$SampleManifest = 'local-analysis/oracle/samples-a2.json'
)
# Use only the source-built, pinned ignored reference library. No protected input.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $pin = (& git -C references/toolkit rev-parse HEAD).Trim()
    if ($pin -ne 'd6a2ef80b5a09999f08f23a803b516096055e5c9') { throw 'Toolkit pin differs' }
    $dirty = & git -C references/toolkit status --porcelain --untracked-files=no
    if ($dirty) { throw 'Toolkit source has modifications; oracle must be unmodified' }
    $javaRoot = (Get-ChildItem -LiteralPath local-analysis/toolchain/java -Directory | Select-Object -First 1).FullName
    $jar = Join-Path $projectRoot 'references/toolkit/lib/cwlib/target/cwlib-0.1.jar'
    if (-not (Test-Path -LiteralPath $jar)) { throw 'Build pinned cwlib first; see docs/scanner.md' }
    $classes = Join-Path $projectRoot 'local-analysis/oracle/classes'
    New-Item -ItemType Directory -Force -Path $classes | Out-Null
    & "$javaRoot/bin/javac.exe" -cp $jar -d $classes scripts/ToolkitOracle.java
    if ($LASTEXITCODE -ne 0) { throw 'Oracle harness compilation failed' }
    & "$javaRoot/bin/java.exe" '-Xmx512m' '-Djava.awt.headless=true' -cp "$classes;$jar" ToolkitOracle `
        game-data/a1-disc/PS3_GAME/USRDIR/OUTPUT/BLURAYGUIDS.MAP `
        game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC $SampleManifest $Output
    if ($LASTEXITCODE -ne 0) { throw 'Oracle execution failed' }
} finally { Pop-Location }
