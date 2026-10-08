param(
    [Parameter(Mandatory=$true)][string]$Trans,
    [Parameter(Mandatory=$true)][string]$Output
)
# U1: Rust translation_dump of one extracted .trans file, then Toolkit parity at the pin.
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if ((& git -C references/toolkit rev-parse HEAD).Trim() -ne 'd6a2ef80b5a09999f08f23a803b516096055e5c9') { throw 'Toolkit pin differs' }
    if (& git -C references/toolkit status --porcelain --untracked-files=no) { throw 'Modified Toolkit source' }
    & cargo run -q -p lbp-resource --example translation_dump -- $Trans $Output
    if ($LASTEXITCODE -ne 0) { throw 'translation_dump failed' }
    $javaRoot = (Get-ChildItem -LiteralPath local-analysis/toolchain/java -Directory | Select-Object -First 1).FullName
    $jar = Join-Path $projectRoot 'references/toolkit/lib/cwlib/target/cwlib-0.1.jar'
    $classes = Join-Path $projectRoot 'local-analysis/oracle/classes'
    New-Item -ItemType Directory -Force -Path $classes | Out-Null
    & "$javaRoot/bin/javac.exe" -cp $jar -d $classes scripts/TranslationOracle.java
    if ($LASTEXITCODE -ne 0) { throw 'Translation oracle compilation failed' }
    & "$javaRoot/bin/java.exe" '-Xmx512m' '-Djava.awt.headless=true' -cp "$classes;$jar" TranslationOracle $Trans $Output
    if ($LASTEXITCODE -ne 0) { throw 'Translation oracle reported differences' }
} finally { Pop-Location }
