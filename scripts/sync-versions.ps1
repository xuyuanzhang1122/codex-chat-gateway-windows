param(
    [string]$ProjectRoot = ''
)

# Sync desktop-tauri versions from the root VERSION file.
# The root VERSION is the single source of truth; tauri.conf.json and
# Cargo.toml must match it or the built console misreports its own version
# (which breaks update checks and the footer display).

$ErrorActionPreference = 'Stop'
if (-not $ProjectRoot) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$version = (Get-Content -LiteralPath (Join-Path $ProjectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Invalid VERSION: $version"
}
$utf8 = [Text.UTF8Encoding]::new($false)

$confPath = Join-Path $ProjectRoot 'desktop-tauri\src-tauri\tauri.conf.json'
$conf = [IO.File]::ReadAllText($confPath)
$confNew = [regex]::Replace(
    $conf,
    '(?m)^(\s*)"version": "[^"]+"',
    { param($m) $m.Groups[1].Value + '"version": "' + $version + '"' },
    1
)
if ($confNew -ne $conf) {
    [IO.File]::WriteAllText($confPath, $confNew, $utf8)
    Write-Host "tauri.conf.json version -> $version"
}

$cargoPath = Join-Path $ProjectRoot 'desktop-tauri\src-tauri\Cargo.toml'
$cargo = [IO.File]::ReadAllText($cargoPath)
$cargoNew = [regex]::Replace($cargo, '(?m)^version = "[^"]+"', "version = `"$version`"", 1)
if ($cargoNew -ne $cargo) {
    [IO.File]::WriteAllText($cargoPath, $cargoNew, $utf8)
    Write-Host "Cargo.toml version -> $version"
}
