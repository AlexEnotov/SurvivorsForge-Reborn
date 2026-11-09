param(
    [string]$ToolRoot = (Join-Path $PSScriptRoot '..\AzerothSurvivorsTest\.local-tools'),
    [switch]$Test
)
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
if (Test-Path -LiteralPath (Join-Path $ToolRoot 'cargo/bin/cargo.exe')) {
    $ToolRoot = (Resolve-Path -LiteralPath $ToolRoot).Path
    $env:RUSTUP_HOME = Join-Path $ToolRoot 'rustup'
    $env:CARGO_HOME = Join-Path $ToolRoot 'cargo'
    $env:PATH = "$ToolRoot\cargo\bin;$ToolRoot\w64devkit\bin;$env:PATH"
}
$env:CARGO_BUILD_JOBS = '6'
if ($Test) {
    $env:SLINT_EMIT_DEBUG_INFO = '1'
    cargo test --locked -p survivorsforge -p forge-core -p forge-scan
} else {
    cargo build --release --locked -p survivorsforge
}
exit $LASTEXITCODE
