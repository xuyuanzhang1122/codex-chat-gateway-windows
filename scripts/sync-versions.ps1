param(
    [string]$ProjectRoot = ''
)

# Sync all product versions from the root VERSION file.
# The root VERSION is the single source of truth; npm, Tauri, and both
# Cargo manifests must match it or the built console misreports its own version
# (which breaks update checks and the footer display).

$ErrorActionPreference = 'Stop'
if (-not $ProjectRoot) {
    $ProjectRoot = Split-Path -Parent $PSScriptRoot
}
$version = (Get-Content -LiteralPath (Join-Path $ProjectRoot 'VERSION') -Raw).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
    throw "Invalid VERSION: $version"
}
$utf8 = [Text.UTF8Encoding]::new($false)

$packagePath = Join-Path $ProjectRoot 'desktop-tauri\package.json'
$package = [IO.File]::ReadAllText($packagePath)
$packageNew = [regex]::Replace(
    $package,
    '("name": "codex-chat-gateway-desktop",\r?\n\s*"private": true,\r?\n\s*)"version": "[^"]+"',
    { param($m) $m.Groups[1].Value + '"version": "' + $version + '"' }
)
if ($packageNew -ne $package) {
    [IO.File]::WriteAllText($packagePath, $packageNew, $utf8)
    Write-Host "package.json version -> $version"
}

$packageLockPath = Join-Path $ProjectRoot 'desktop-tauri\package-lock.json'
$packageLock = [IO.File]::ReadAllText($packageLockPath)
$packageLockNew = [regex]::Replace(
    $packageLock,
    '("name": "codex-chat-gateway-desktop",\r?\n\s*)"version": "[^"]+"',
    { param($m) $m.Groups[1].Value + '"version": "' + $version + '"' }
)
if ($packageLockNew -ne $packageLock) {
    [IO.File]::WriteAllText($packageLockPath, $packageLockNew, $utf8)
    Write-Host "package-lock.json version -> $version"
}

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

$nativeCargoPath = Join-Path $ProjectRoot 'native-gateway\Cargo.toml'
$nativeCargo = [IO.File]::ReadAllText($nativeCargoPath)
$nativeCargoNew = [regex]::Replace($nativeCargo, '(?m)^version = "[^"]+"', "version = `"$version`"", 1)
if ($nativeCargoNew -ne $nativeCargo) {
    [IO.File]::WriteAllText($nativeCargoPath, $nativeCargoNew, $utf8)
    Write-Host "native-gateway Cargo.toml version -> $version"
}
