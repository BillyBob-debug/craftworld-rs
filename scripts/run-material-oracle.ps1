param(
    [Parameter(Mandatory=$true)][string]$Output,
    [string]$Archive = 'game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC',
    [string]$SampleManifest = 'local-analysis/d3-material-samples.json',
    [switch]$Graph,
    [switch]$Tail,
    [switch]$Diffuse
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
    & "$javaRoot/bin/javac.exe" -cp $jar -d $classes scripts/TextureOracle.java scripts/MaterialOracle.java scripts/MaterialGraphOracle.java scripts/MaterialTailOracle.java
    if ($LASTEXITCODE -ne 0) { throw 'Material oracle compilation failed' }
    $graphArguments = @()
    if ($Graph) { $graphArguments = @('--graph') }
    if ($Tail) { $graphArguments = @('--tail') }
    if ($Diffuse) { $graphArguments = @('--diffuse') }
    & "$javaRoot/bin/java.exe" '-Xmx256m' '-Djava.awt.headless=true' -cp "$classes;$jar" MaterialOracle `
        $Archive $SampleManifest $Output @graphArguments
    if ($LASTEXITCODE -ne 0) { throw 'Material oracle execution failed' }
} finally { Pop-Location }
