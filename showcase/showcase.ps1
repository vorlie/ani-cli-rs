$ErrorActionPreference = 'Stop'

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$OutputDirectory = Join-Path $PSScriptRoot 'output'
$ScreenshotDirectory = Join-Path $OutputDirectory 'screenshots'
$DocumentationAssets = Join-Path $ProjectRoot 'docs\assets'

if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
    throw 'WSL is required for deterministic VHS recording on Windows.'
}

if (Test-Path -LiteralPath $OutputDirectory) {
    $ResolvedOutput = (Resolve-Path -LiteralPath $OutputDirectory).Path
    $ResolvedShowcase = (Resolve-Path -LiteralPath $PSScriptRoot).Path
    if (-not $ResolvedOutput.StartsWith($ResolvedShowcase, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clear output outside the showcase directory: $ResolvedOutput"
    }
    Remove-Item -LiteralPath $ResolvedOutput -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $OutputDirectory, $ScreenshotDirectory, $DocumentationAssets | Out-Null

$LinuxProjectRoot = (& wsl.exe -e wslpath -a $ProjectRoot).Trim()
if ($LASTEXITCODE -ne 0 -or -not $LinuxProjectRoot) {
    throw 'Could not translate the project directory into a WSL path.'
}

$LinuxTools = "$LinuxProjectRoot/showcase/.tools/linux"
$Bootstrap = @"
set -euo pipefail
project='$LinuxProjectRoot'
tools='$LinuxTools'
mkdir -p "`$tools"

if ! command -v cargo >/dev/null; then
  echo 'Rust/Cargo is required inside WSL.' >&2
  exit 1
fi
if ! command -v ffmpeg >/dev/null; then
  echo 'FFmpeg is required inside WSL.' >&2
  exit 1
fi
if ! command -v curl >/dev/null; then
  echo 'curl is required inside WSL to install pinned showcase tools.' >&2
  exit 1
fi

if [ ! -x "`$tools/vhs" ]; then
  curl -fsSL 'https://github.com/charmbracelet/vhs/releases/download/v0.11.0/vhs_0.11.0_Linux_x86_64.tar.gz' -o "`$tools/vhs.tar.gz"
  echo '99cb634587eaae0473c1ea377db80c3a048c27f99fe0a7febb1a1e8cb7ee5009  '"`$tools"'/vhs.tar.gz' | sha256sum -c -
  tar -xzf "`$tools/vhs.tar.gz" -C "`$tools"
  cp "`$tools/vhs_0.11.0_Linux_x86_64/vhs" "`$tools/vhs"
  chmod +x "`$tools/vhs"
fi

if [ ! -x "`$tools/ttyd" ]; then
  curl -fsSL 'https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64' -o "`$tools/ttyd"
  echo '8a217c968aba172e0dbf3f34447218dc015bc4d5e59bf51db2f2cd12b7be4f55  '"`$tools"'/ttyd' | sha256sum -c -
  chmod +x "`$tools/ttyd"
fi

cd "`$project"
cargo build
PATH="`$tools:`$PATH" "`$tools/vhs" showcase/ani-cli-rs.tape

mp4='showcase/output/ani-cli-rs-showcase.mp4'
gif='docs/assets/ani-cli-rs-showcase.gif'
duration="`$(ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 "`$mp4")"
dimensions="`$(ffprobe -v error -select_streams v:0 -show_entries stream=width,height -of csv=s=x:p=0 "`$mp4")"
awk -v value="`$duration" 'BEGIN { exit !(value >= 60 && value <= 90) }' || {
  echo "Expected a 60-90 second MP4, got `${duration}s." >&2
  exit 1
}
[ "`$dimensions" = '1280x720' ] || {
  echo "Expected a 1280x720 MP4, got `$dimensions." >&2
  exit 1
}
[ "`$(stat -c %s "`$gif")" -le 10485760 ] || {
  echo 'README GIF exceeds the 10 MiB limit.' >&2
  exit 1
}
"@

& wsl.exe -e bash -lc $Bootstrap
if ($LASTEXITCODE -ne 0) {
    throw "WSL showcase generation failed with exit code $LASTEXITCODE"
}

$Mp4 = Join-Path $OutputDirectory 'ani-cli-rs-showcase.mp4'
$Gif = Join-Path $DocumentationAssets 'ani-cli-rs-showcase.gif'
foreach ($Output in @($Mp4, $Gif)) {
    if (-not (Test-Path -LiteralPath $Output)) { throw "Expected showcase output was not created: $Output" }
    if ((Get-Item -LiteralPath $Output).Length -eq 0) { throw "Showcase output is empty: $Output" }
}

Write-Host ''
Write-Host 'ani-cli-rs showcase complete.'
Write-Host "MP4: $Mp4"
Write-Host "GIF: $Gif"
