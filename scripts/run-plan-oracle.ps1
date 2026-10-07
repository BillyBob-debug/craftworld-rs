param(
    [Parameter(Mandatory=$true)][string]$Output,
    [string]$Archive = 'game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC',
    [string]$SampleManifest = 'local-analysis/d27-plan-samples.json'
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if ((& git -C references/toolkit rev-parse HEAD).Trim() -ne 'd6a2ef80b5a09999f08f23a803b516096055e5c9') { throw 'Toolkit pin differs' }
    if (& git -C references/toolkit status --porcelain --untracked-files=no) { throw 'Modified Toolkit source' }
    $javaRoot = (Get-ChildItem -LiteralPath local-analysis/toolchain/java -Directory | Select-Object -First 1).FullName
    $jar = Join-Path $projectRoot 'references/toolkit/lib/cwlib/target/cwlib-0.1.jar'
    $classes = Join-Path $projectRoot 'local-analysis/oracle/classes'
    New-Item -ItemType Directory -Force -Path $classes | Out-Null
    & "$javaRoot/bin/javac.exe" -cp $jar -d $classes scripts/TextureOracle.java scripts/PlanOracle.java
    if ($LASTEXITCODE -ne 0) { throw 'Plan oracle compilation failed' }
    & "$javaRoot/bin/java.exe" '-Xmx256m' '-Djava.awt.headless=true' -cp "$classes;$jar" PlanOracle --self-test
    if ($LASTEXITCODE -ne 0) { throw 'Plan synthetic guards failed' }
    & "$javaRoot/bin/java.exe" '-Xmx256m' '-Djava.awt.headless=true' -cp "$classes;$jar" PlanOracle `
        $Archive $SampleManifest $Output
    if ($LASTEXITCODE -ne 0) { throw 'Plan oracle failed' }
} finally { Pop-Location }
