param(
    [string]$InstallDirectory = (Join-Path $env:LOCALAPPDATA "Programs\ani-cli-rs\bin"),
    [switch]$NoPathUpdate
)

$ErrorActionPreference = "Stop"
$repository = "vorlie/ani-cli-rs"
$target = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    "X64" { "x86_64-pc-windows-msvc" }
    "Arm64" { "aarch64-pc-windows-msvc" }
    default { throw "ani-cli-rs does not publish a Windows build for this architecture." }
}

$headers = @{ Accept = "application/vnd.github+json"; "User-Agent" = "ani-cli-rs-installer" }
$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repository/releases/latest" -Headers $headers
$version = $release.tag_name.TrimStart("v")
$assetName = "ani-cli-rs-$version-$target.zip"
$asset = $release.assets | Where-Object name -eq $assetName | Select-Object -First 1
$checksumAsset = $release.assets | Where-Object name -eq "$assetName.sha256" | Select-Object -First 1
if (-not $asset -or -not $checksumAsset) {
    throw "The latest release does not contain $assetName and its checksum."
}

$temporaryDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ("ani-cli-rs-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $temporaryDirectory | Out-Null
try {
    $archivePath = Join-Path $temporaryDirectory $assetName
    $checksumPath = "$archivePath.sha256"
    Invoke-WebRequest -Uri $asset.browser_download_url -Headers $headers -OutFile $archivePath
    Invoke-WebRequest -Uri $checksumAsset.browser_download_url -Headers $headers -OutFile $checksumPath

    $expectedHash = ((Get-Content -LiteralPath $checksumPath -Raw).Trim() -split "\s+")[0]
    $actualHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
    if ($expectedHash -ne $actualHash) { throw "Checksum verification failed for $assetName." }

    $expandedDirectory = Join-Path $temporaryDirectory "expanded"
    Expand-Archive -LiteralPath $archivePath -DestinationPath $expandedDirectory
    $binary = Get-ChildItem -LiteralPath $expandedDirectory -Filter "ani-cli-rs.exe" -File -Recurse | Select-Object -First 1
    if (-not $binary) { throw "The release archive did not contain ani-cli-rs.exe." }

    New-Item -ItemType Directory -Force $InstallDirectory | Out-Null
    Copy-Item -LiteralPath $binary.FullName -Destination (Join-Path $InstallDirectory "ani-cli-rs.exe") -Force
} finally {
    if (Test-Path -LiteralPath $temporaryDirectory) { Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force }
}

if (-not $NoPathUpdate) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @($userPath -split ";" | Where-Object { $_ })
    if (-not ($pathEntries | Where-Object { $_.TrimEnd("\") -ieq $InstallDirectory.TrimEnd("\") })) {
        $newPath = (@($pathEntries) + $InstallDirectory) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        $env:Path = "$InstallDirectory;$env:Path"
        Write-Host "Added $InstallDirectory to the user PATH. Open a new terminal to use it there."
    }
}

Write-Host "Installed ani-cli-rs $($release.tag_name) to $InstallDirectory\ani-cli-rs.exe"

