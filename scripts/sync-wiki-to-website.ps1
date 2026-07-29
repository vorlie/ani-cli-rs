param(
    [switch]$DryRun
)

$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Src = Join-Path $Root wiki
$Dst = Join-Path $Root website\docs

$mapping = @{
    'Home' = 'index.md'
    'Home.md' = 'index.md'
    'Installation' = 'guides/installation.md'
    'Installation.md' = 'guides/installation.md'
    'Getting Started' = 'guides/getting-started.md'
    'Getting-Started.md' = 'guides/getting-started.md'
    'Playback and Players' = 'guides/playback-and-players.md'
    'Playback-and-Players.md' = 'guides/playback-and-players.md'
    'Downloads' = 'guides/downloads.md'
    'Downloads.md' = 'guides/downloads.md'
    'Configuration and History' = 'guides/configuration.md'
    'Configuration-and-History.md' = 'guides/configuration.md'
    'CLI Reference' = 'reference/cli.md'
    'CLI-Reference.md' = 'reference/cli.md'
    'FAQ' = 'faq.md'
    'FAQ.md' = 'faq.md'
    'Troubleshooting' = 'support/troubleshooting.md'
    'Troubleshooting.md' = 'support/troubleshooting.md'
    'Provider Architecture' = 'development/architecture.md'
    'Provider-Architecture.md' = 'development/architecture.md'
    'Security and Privacy' = 'development/security.md'
    'Security-and-Privacy.md' = 'development/security.md'
    'Building and Releasing' = 'development/building.md'
    'Building-and-Releasing.md' = 'development/building.md'
    'Contributing' = 'development/contributing.md'
    'Contributing.md' = 'development/contributing.md'
}

$fileMap = @{
    'Home.md' = 'index.md'
    'Installation.md' = 'guides/installation.md'
    'Getting-Started.md' = 'guides/getting-started.md'
    'Playback-and-Players.md' = 'guides/playback-and-players.md'
    'Downloads.md' = 'guides/downloads.md'
    'Configuration-and-History.md' = 'guides/configuration.md'
    'CLI-Reference.md' = 'reference/cli.md'
    'FAQ.md' = 'faq.md'
    'Troubleshooting.md' = 'support/troubleshooting.md'
    'Provider-Architecture.md' = 'development/architecture.md'
    'Security-and-Privacy.md' = 'development/security.md'
    'Building-and-Releasing.md' = 'development/building.md'
    'Contributing.md' = 'development/contributing.md'
}

function Get-RelativePath {
    param(
        [Parameter(Mandatory)] [string]$From,
        [Parameter(Mandatory)] [string]$To
    )
    $uriFrom = New-Object System.Uri((Resolve-Path $From).ProviderPath.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar)
    $uriTo = New-Object System.Uri((Resolve-Path $To).ProviderPath)
    $relative = $uriFrom.MakeRelativeUri($uriTo).ToString()
    return $relative -replace '/', '/'
}

$regex = '\[([^\]]+)\]\(([^)]+)\)'

foreach ($entry in $fileMap.GetEnumerator()) {
    $srcFile = Join-Path $Src $entry.Key
    if (-not (Test-Path $srcFile)) {
        Write-Warning "Source file not found: $srcFile"
        continue
    }

    $destFile = Join-Path $Dst $entry.Value
    $destDir = Split-Path -Parent $destFile
    if (-not (Test-Path $destDir)) {
        if (-not $DryRun) { New-Item -ItemType Directory -Path $destDir | Out-Null }
    }

    $text = Get-Content -Path $srcFile -Raw
    $text = [regex]::Replace($text, $regex, {
        param($m)
        $label = $m.Groups[1].Value
        $target = $m.Groups[2].Value

        if ($target -match '^[a-zA-Z]+://') {
            return $m.Value
        }
        if ($target.StartsWith('#')) {
            return $m.Value
        }
        if ($target.StartsWith('/')) {
            return $m.Value
        }

        $parts = $target -split '#', 2
        $base = $parts[0]
        $anchor = if ($parts.Count -gt 1) { $parts[1] } else { '' }
        $key = if ($base) { $base } else { $target }

        $targetRel = $null
        if ($mapping.ContainsKey($key)) {
            $targetRel = $mapping[$key]
        } elseif ($mapping.ContainsKey($key + '.md')) {
            $targetRel = $mapping[$key + '.md']
        } elseif ($mapping.ContainsKey($key.ToLower())) {
            $targetRel = $mapping[$key.ToLower()]
        }

        if (-not $targetRel) {
            return $m.Value
        }

        $targetPath = Join-Path $Dst $targetRel
        $sourceDir = Split-Path -Parent (Join-Path $Dst $entry.Value)
        $relativePath = Get-RelativePath -From $sourceDir -To $targetPath
        if ($anchor) { $relativePath = "$relativePath#$anchor" }
        return "[$label]($relativePath)"
    })

    if (-not $DryRun) {
        Set-Content -Path $destFile -Value $text -NoNewline
        Write-Host "copied: $($entry.Key) -> $($entry.Value)"
    } else {
        Write-Host "dry-run: $($entry.Key) -> $($entry.Value)"
    }
}
