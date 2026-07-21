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

$marker = Join-Path $runtimeRoot 'CCG_RUNTIME_PROFILE'
$text = "openai-compatible-minimal`npruned_files=$removedFiles`npruned_bytes=$removedBytes`n"
[IO.File]::WriteAllText($marker, $text, [Text.UTF8Encoding]::new($false))
$removedMb = [math]::Round($removedBytes / 1MB, 1)
Write-Host "Pruned optional runtime packages: $removedFiles files, $removedMb MB"
