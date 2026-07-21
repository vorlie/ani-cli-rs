param(
    [string]$InstallDirectory = $(if ($env:ANI_CLI_RS_INSTALL_DIR) { $env:ANI_CLI_RS_INSTALL_DIR } else { Join-Path ([Environment]::GetFolderPath("UserProfile")) ".local\bin" })
)

$ErrorActionPreference = "Stop"
$binaryPath = Join-Path $InstallDirectory "ani-cli-rs.exe"
if (Test-Path -LiteralPath $binaryPath) {
    Remove-Item -LiteralPath $binaryPath -Force
    Write-Host "Removed $binaryPath"
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @($userPath -split ";" | Where-Object { $_ -and $_.TrimEnd("\") -ine $InstallDirectory.TrimEnd("\") })
[Environment]::SetEnvironmentVariable("Path", ($pathEntries -join ";"), "User")
Write-Host "ani-cli-rs has been uninstalled. Open a new terminal to refresh PATH."
