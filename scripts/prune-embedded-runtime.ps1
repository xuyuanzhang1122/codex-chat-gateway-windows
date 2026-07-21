param(
    [Parameter(Mandatory = $true)][string]$RuntimeDir
)

# Keep the embedded runtime focused on the OpenAI-compatible gateway paths
# exposed by this product. LiteLLM's full [proxy] extra also installs optional
# analytics, cloud-provider, alternate-server, and audio stacks that are not
# used by config.yaml but add hundreds of MB and thousands of antivirus scans.

$ErrorActionPreference = 'Stop'
$runtimeRoot = [IO.Path]::GetFullPath($RuntimeDir)
$sitePackages = Join-Path $runtimeRoot 'Lib\site-packages'
if (-not (Test-Path -LiteralPath (Join-Path $runtimeRoot 'python.exe'))) {
    throw "Embedded Python runtime is missing: $runtimeRoot"
}
if (-not (Test-Path -LiteralPath $sitePackages)) {
    throw "site-packages is missing: $sitePackages"
}

$siteRoot = [IO.Path]::GetFullPath($sitePackages).TrimEnd('\') + '\'
$patterns = @(
    '_polars_runtime_32',
    'polars',
    'polars-*.dist-info',
    'polars_runtime_32-*.dist-info',
    'granian',
    'granian-*.dist-info',
    'bin',
    'pythonwin',
    'numpy',
    'numpy.libs',
    'numpy-*.dist-info',
    'hf_xet',
    'hf_xet-*.dist-info',
    'mcp',
    'mcp-*.dist-info',
    'gunicorn',
    'gunicorn-*.dist-info',
    'rq',
    'rq-*.dist-info',
    'win32',
    'win32com',
    'win32comext',
    'adodbapi',
    'isapi',
    'PyWin32.chm',
    'pythoncom.py',
    'pywintypes.py',
    'pywin32_system32',
    'pywin32-*.dist-info',
    'pywin32.pth',
    'pywin32.version.txt',
    'azure',
    'azure_*.dist-info',
    'soundfile.py',
    'soundfile-*.dist-info',
    '_soundfile.py',
    '_soundfile_data'
)

$removedBytes = [int64]0
$removedFiles = 0
foreach ($pattern in $patterns) {
    $items = @(Get-ChildItem -LiteralPath $sitePackages -Filter $pattern -Force -ErrorAction SilentlyContinue)
    foreach ($item in $items) {
        $fullPath = [IO.Path]::GetFullPath($item.FullName)
        if (-not $fullPath.StartsWith($siteRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to prune path outside site-packages: $fullPath"
        }
        if ($item.PSIsContainer) {
            $measure = Get-ChildItem -LiteralPath $fullPath -Recurse -File -Force -ErrorAction SilentlyContinue |
                Measure-Object -Property Length -Sum
            $removedFiles += $measure.Count
            $removedBytes += [int64]$measure.Sum
            Remove-Item -LiteralPath $fullPath -Recurse -Force
        }
        else {
            $removedFiles += 1
            $removedBytes += [int64]$item.Length
            Remove-Item -LiteralPath $fullPath -Force
        }
    }
}

# Wheel test suites and development fixtures are never imported by the
# gateway. Remove deepest paths first so nested test directories are counted
# once and cannot leave empty parent fixtures behind.
$testDirectories = @(
    Get-ChildItem -LiteralPath $sitePackages -Recurse -Directory -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -eq 'test' -or $_.Name -eq 'tests' } |
        Sort-Object { $_.FullName.Length } -Descending
)
foreach ($item in $testDirectories) {
    if (-not (Test-Path -LiteralPath $item.FullName)) {
        continue
    }
    $fullPath = [IO.Path]::GetFullPath($item.FullName)
    if (-not $fullPath.StartsWith($siteRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to prune test path outside site-packages: $fullPath"
    }
    $measure = Get-ChildItem -LiteralPath $fullPath -Recurse -File -Force -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum
    $removedFiles += $measure.Count
    $removedBytes += [int64]$measure.Sum
    Remove-Item -LiteralPath $fullPath -Recurse -Force
}

$developmentDirectories = @(
    Get-ChildItem -LiteralPath $sitePackages -Recurse -Directory -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @('example', 'examples', 'example_config_yaml') } |
        Sort-Object { $_.FullName.Length } -Descending
)
foreach ($item in $developmentDirectories) {
    if (-not (Test-Path -LiteralPath $item.FullName)) {
        continue
    }
    $fullPath = [IO.Path]::GetFullPath($item.FullName)
    if (-not $fullPath.StartsWith($siteRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to prune development path outside site-packages: $fullPath"
    }
    $measure = Get-ChildItem -LiteralPath $fullPath -Recurse -File -Force -ErrorAction SilentlyContinue |
        Measure-Object -Property Length -Sum
    $removedFiles += $measure.Count
    $removedBytes += [int64]$measure.Sum
    Remove-Item -LiteralPath $fullPath -Recurse -Force
}

$bytecodeFiles = @(Get-ChildItem -LiteralPath $sitePackages -Recurse -File -Filter '*.pyc' -Force -ErrorAction SilentlyContinue)
foreach ($item in $bytecodeFiles) {
    $removedFiles += 1
    $removedBytes += [int64]$item.Length
    Remove-Item -LiteralPath $item.FullName -Force
}
$cacheDirectories = @(
    Get-ChildItem -LiteralPath $sitePackages -Recurse -Directory -Filter '__pycache__' -Force -ErrorAction SilentlyContinue |
        Sort-Object { $_.FullName.Length } -Descending
)
foreach ($item in $cacheDirectories) {
    if (Test-Path -LiteralPath $item.FullName) {
        Remove-Item -LiteralPath $item.FullName -Recurse -Force
    }
}

$marker = Join-Path $runtimeRoot 'CCG_RUNTIME_PROFILE'
$text = "openai-compatible-minimal`npruned_files=$removedFiles`npruned_bytes=$removedBytes`n"
[IO.File]::WriteAllText($marker, $text, [Text.UTF8Encoding]::new($false))
$removedMb = [math]::Round($removedBytes / 1MB, 1)
Write-Host "Pruned optional runtime packages: $removedFiles files, $removedMb MB"
