param(
    [Parameter(Mandatory=$true)][string]$Output,
    [string]$Archive = 'game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC',
    [string]$SampleManifest = 'local-analysis/d1-mesh-samples.json',
    [switch]$Uv,
    [switch]$Groups
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    $pin = (& git -C references/toolkit rev-parse HEAD).Trim()
    if ($pin -ne 'd6a2ef80b5a09999f08f23a803b516096055e5c9') { throw 'Toolkit pin differs' }
    if (& git -C references/toolkit status --porcelain --untracked-files=no) { throw 'Modified Toolkit source' }
    $javaRoot = (Get-ChildItem -LiteralPath local-analysis/toolchain/java -Directory | Select-Object -First 1).FullName
    $jar = Join-Path $projectRoot 'references/toolkit/lib/cwlib/target/cwlib-0.1.jar'
    $classes = Join-Path $projectRoot 'local-analysis/oracle/classes'
    New-Item -ItemType Directory -Force -Path $classes | Out-Null
    & "$javaRoot/bin/javac.exe" -cp $jar -d $classes scripts/TextureOracle.java scripts/MeshOracle.java
    if ($LASTEXITCODE -ne 0) { throw 'Mesh oracle compilation failed' }
    $oracleArgs = @($Archive, $SampleManifest, $Output)
    if ($Uv -and $Groups) { throw 'Choose UV or groups' }
    if ($Uv) { $oracleArgs += '--uv' }
    if ($Groups) { $oracleArgs += '--groups' }
    & "$javaRoot/bin/java.exe" '-Xmx256m' '-Djava.awt.headless=true' -cp "$classes;$jar" MeshOracle `
        @oracleArgs
    if ($LASTEXITCODE -ne 0) { throw 'Mesh oracle execution failed' }
} finally { Pop-Location }
