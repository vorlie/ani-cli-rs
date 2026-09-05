param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string[]]$Features = @()
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$previousRustFlags = $env:RUSTFLAGS
$previousEncodedRustFlags = $env:CARGO_ENCODED_RUSTFLAGS
$profilePath = $env:USERPROFILE

if (-not $profilePath) {
    throw "USERPROFILE is unavailable; cannot hide the local build path."
}

$separator = [char]0x1f
$remapFlag = "--remap-path-prefix=$profilePath=/build"
if (-not [string]::IsNullOrEmpty($previousEncodedRustFlags)) {
    $flags = @($previousEncodedRustFlags -split $separator)
}
else {
    $flags = @($previousRustFlags -split "\s+" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}
$env:RUSTFLAGS = $null
$env:CARGO_ENCODED_RUSTFLAGS = (@($flags) + $remapFlag) -join $separator

try {
    $cargoArgs = @(
        "build",
        "--locked",
        "--release",
        "--target",
        $Target,
        "--manifest-path",
        (Join-Path $projectRoot "Cargo.toml")
    )

    if ($Features -and $Features.Count -gt 0) {
        $cargoArgs += @("--features")
        $cargoArgs += $Features
    }

    Write-Host "Building $Target with features: $($Features -join ', ')"
    cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed for $Target."
    }
}
finally {
    $env:RUSTFLAGS = $previousRustFlags
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags
}
