param(
    [Parameter(Mandatory = $true)][string]$DestinationRuntimeDir,
    [string]$BuildPython = ''
)

# Build ONLY the embedded CPython + LiteLLM site-packages tree.
# Never compiles the legacy C#/WPF desktop (that is build-desktop.ps1 / build-portable.ps1).

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$dest = [IO.Path]::GetFullPath($DestinationRuntimeDir)

$buildCache = Join-Path $projectRoot '.portable-build'
$pythonArchive = Join-Path $buildCache 'python-3.11.9-embed-amd64.zip'
New-Item -ItemType Directory -Path $buildCache -Force | Out-Null
if (-not (Test-Path -LiteralPath $pythonArchive)) {
    Write-Host 'Downloading official CPython 3.11.9 embeddable x64...'
    Invoke-WebRequest -Uri 'https://www.python.org/ftp/python/3.11.9/python-3.11.9-embed-amd64.zip' -OutFile $pythonArchive -UseBasicParsing
}
$pythonHash = (Get-FileHash -LiteralPath $pythonArchive -Algorithm SHA256).Hash
if ($pythonHash -ne '009D6BF7E3B2DDCA3D784FA09F90FE54336D5B60F0E0F305C37F400BF83CFD3B') {
    throw "Unexpected embedded Python SHA-256: $pythonHash"
}

if (Test-Path -LiteralPath $dest) {
    Remove-Item -LiteralPath $dest -Recurse -Force
}
New-Item -ItemType Directory -Path $dest -Force | Out-Null
Expand-Archive -LiteralPath $pythonArchive -DestinationPath $dest -Force
$pth = Join-Path $dest 'python311._pth'
[IO.File]::WriteAllLines(
    $pth,
    @('python311.zip', '.', 'Lib\site-packages', '', 'import site'),
    [Text.UTF8Encoding]::new($false)
)
$sitePackages = Join-Path $dest 'Lib\site-packages'
New-Item -ItemType Directory -Path $sitePackages -Force | Out-Null

if (-not $BuildPython) {
    $projectPython = Join-Path $projectRoot '.venv\Scripts\python.exe'
    if (Test-Path -LiteralPath $projectPython) {
        $BuildPython = $projectPython
    }
    else {
        $BuildPython = (Get-Command python -ErrorAction Stop).Source
    }
}
& $BuildPython -m pip --version *> $null
if ($LASTEXITCODE -ne 0) {
    throw "Build Python does not provide pip: $BuildPython"
}

Write-Host "Installing LiteLLM into embedded runtime via $BuildPython ..."
& $BuildPython -m pip install --disable-pip-version-check --quiet --target $sitePackages -r (Join-Path $projectRoot 'requirements.txt')
if ($LASTEXITCODE -ne 0) {
    throw 'Dependency installation failed.'
}

if (-not (Test-Path -LiteralPath (Join-Path $dest 'python.exe'))) {
    throw "Embedded runtime incomplete: $dest"
}
Write-Host "Embedded runtime ready: $dest"
