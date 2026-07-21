param(
    [Parameter(Mandatory = $true)][string]$Version
)

# Extract the release notes for $Version from CHANGELOG.md.
# Falls back to the Unreleased section, then to a generic one-liner.

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$changelogPath = Join-Path $projectRoot 'CHANGELOG.md'

$notes = ''
if (Test-Path -LiteralPath $changelogPath) {
    $changelog = Get-Content -LiteralPath $changelogPath -Raw -Encoding UTF8
    $escaped = [regex]::Escape($Version)
    if ($changelog -match "(?ms)^##\s+\[?$escaped\]?[^\n]*\n(.*?)(?=^##\s|\z)") {
        $notes = $Matches[1].Trim()
    }
    if (-not $notes -and $changelog -match "(?ms)^##\s+Unreleased[^\n]*\n(.*?)(?=^##\s|\z)") {
        $notes = $Matches[1].Trim()
    }
}
if (-not $notes) {
    $notes = '- Studio console (Tauri 2 + LobeHub)'
}
Write-Output $notes
