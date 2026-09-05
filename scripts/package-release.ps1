param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [string[]]$Features = @()
)

$ErrorActionPreference = "Stop"

function Get-Sha256Hash {
    param([Parameter(Mandatory = $true)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            return ([System.BitConverter]::ToString($sha256.ComputeHash($stream))).Replace("-", "")
        }
        finally {
            $sha256.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

$projectRoot = Split-Path -Parent $PSScriptRoot
$metadata = cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $projectRoot "Cargo.toml") | ConvertFrom-Json
$version = $metadata.packages[0].version
$binaryName = if ($Target -like "*windows*") { "ani-cli-rs.exe" } else { "ani-cli-rs" }
$binaryPath = Join-Path $projectRoot "target/$Target/release/$binaryName"
$binaryNames = @($binaryName)
$guiFeatureEnabled = $Features -and ($Features | Where-Object { $_ -eq "gui" -or $_ -like "gui,*" -or $_ -like "*,gui" }).Count -gt 0
if ($guiFeatureEnabled) {
    $guiBinaryName = if ($Target -like "*windows*") { "ani-cli-rs-gui.exe" } else { "ani-cli-rs-gui" }
    $guiBinaryPath = Join-Path $projectRoot "target/$Target/release/$guiBinaryName"
    if (-not (Test-Path -LiteralPath $guiBinaryPath)) {
        throw "GUI binary not found: $guiBinaryPath"
    }
    $binaryNames += $guiBinaryName
}
$packageName = "ani-cli-rs-$version-$Target"
$packageDirectory = Join-Path $projectRoot "dist/$packageName"
$archivePath = Join-Path $projectRoot "dist/$packageName.zip"
$licenseCandidates = @(
    (Join-Path $projectRoot "LICENSE"),
    (Join-Path $projectRoot "../../LICENSE")
)
$licensePath = $licenseCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $licensePath) {
    throw "Could not find LICENSE in the project or repository root."
}

if ($Target -like "*windows*") {
    & (Join-Path $PSScriptRoot "build-windows.ps1") -Target $Target -Features $Features
}
else {
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

New-Item -ItemType Directory -Force $packageDirectory | Out-Null
foreach ($artifactName in $binaryNames) {
    $artifactPath = Join-Path $projectRoot "target/$Target/release/$artifactName"
    if (-not (Test-Path -LiteralPath $artifactPath)) {
        throw "Expected release binary not found: $artifactPath"
    }
    Copy-Item -LiteralPath $artifactPath -Destination (Join-Path $packageDirectory $artifactName) -Force
}
Copy-Item -LiteralPath (Join-Path $projectRoot "README.md") -Destination (Join-Path $packageDirectory "README.md") -Force
Copy-Item -LiteralPath $licensePath -Destination (Join-Path $packageDirectory "LICENSE") -Force

Compress-Archive -Path "$packageDirectory/*" -DestinationPath $archivePath -Force
$archiveHash = (Get-Sha256Hash -Path $archivePath).ToLowerInvariant()
$checksumPath = "$archivePath.sha256"
Set-Content -LiteralPath $checksumPath -Value "$archiveHash  $([System.IO.Path]::GetFileName($archivePath))" -Encoding ascii
Write-Host "Created $archivePath and $checksumPath"

# Only run Inno Setup for Windows targets
if ($Target -like "*windows*") {
    $issPath = Join-Path $projectRoot "installer.iss"

    # Locate ISCC.exe (check PATH, or default install paths)
    $iscc = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    if (-not $iscc) {
        $defaultPaths = @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
        )
        $iscc = $defaultPaths | Where-Object { Test-Path $_ } | Select-Object -First 1
    }

    if (-not $iscc) {
        throw "Inno Setup Compiler (ISCC.exe) not found. Please install it or add it to PATH."
    }

    Write-Host "Building Inno Setup installer..."
    & $iscc "/DAppVersion=$version" "/DSourceDir=$packageDirectory" "/DOutputDir=$(Join-Path $projectRoot 'dist')" $issPath
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup compilation failed."
    }

    $setupName = "ani-cli-rs-$version-windows-x64-setup.exe"
    $setupPath = Join-Path $projectRoot "dist/$setupName"
    if (-not (Test-Path -LiteralPath $setupPath)) {
        throw "Inno Setup did not create the expected $setupName asset."
    }
    $setupHash = (Get-Sha256Hash -Path $setupPath).ToLowerInvariant()
    $setupChecksumPath = "$setupPath.sha256"
    Set-Content -LiteralPath $setupChecksumPath -Value "$setupHash  $setupName" -Encoding ascii
    Write-Host "Created $setupPath and $setupChecksumPath"
}
