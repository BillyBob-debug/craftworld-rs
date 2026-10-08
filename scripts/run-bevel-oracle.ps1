param(
    [Parameter(Mandatory=$true)][string]$Output,
    [Parameter(Mandatory=$true)][string]$SampleManifest,
    [string]$DiscArchive = 'game-data/a1-disc/PS3_GAME/USRDIR/DATA.FARC',
    [string]$PatchDirectory = 'game-data/a1-update-0133/USRDIR/patches',
    # Data root (references/, local-analysis/, game-data/). Defaults to the checkout holding this script.
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)
# D122: pinned Toolkit RBevel fields for every manifest bevel (scripts/bevel_report.py manifest).
$ErrorActionPreference = 'Stop'
$scripts = $PSScriptRoot
Push-Location $ProjectRoot
try {
    if ((& git -C references/toolkit rev-parse HEAD).Trim() -ne 'd6a2ef80b5a09999f08f23a803b516096055e5c9') { throw 'Toolkit pin differs' }
    if (& git -C references/toolkit status --porcelain --untracked-files=no) { throw 'Modified Toolkit source' }
    $javaRoot = (Get-ChildItem -LiteralPath local-analysis/toolchain/java -Directory | Select-Object -First 1).FullName
    $jar = Join-Path (Get-Location) 'references/toolkit/lib/cwlib/target/cwlib-0.1.jar'
    $classes = Join-Path (Get-Location) 'local-analysis/oracle/bevel-classes'
    New-Item -ItemType Directory -Force -Path $classes | Out-Null
    & "$javaRoot/bin/javac.exe" -cp $jar -d $classes (Join-Path $scripts 'BevelOracle.java')
    if ($LASTEXITCODE -ne 0) { throw 'Bevel oracle compilation failed' }
    & "$javaRoot/bin/java.exe" '-Xmx256m' '-Djava.awt.headless=true' -cp "$classes;$jar" BevelOracle --self-test
    if ($LASTEXITCODE -ne 0) { throw 'Bevel synthetic guards failed' }
    $archives = @($DiscArchive) + @(Get-ChildItem -LiteralPath $PatchDirectory -Filter 'patch_*.farc' | Sort-Object Name | ForEach-Object { $_.FullName })
    & "$javaRoot/bin/java.exe" '-Xmx256m' '-Djava.awt.headless=true' -cp "$classes;$jar" BevelOracle `
        $SampleManifest $Output @archives
    if ($LASTEXITCODE -ne 0) { throw 'Bevel oracle failed' }
} finally { Pop-Location }
