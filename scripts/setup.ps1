$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$venv = Join-Path $projectRoot '.venv'
$requirements = Join-Path $projectRoot 'requirements.txt'
$envFile = Join-Path $projectRoot '.env'
$envExample = Join-Path $projectRoot '.env.example'

if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    throw 'Python was not found. Python 3.10-3.13 is required.'
}

$versionText = & python --version 2>&1
if ($LASTEXITCODE -ne 0 -or $versionText -notmatch 'Python (3\.(1[0-3]))\.') {
    throw "Unsupported Python version: $versionText. Python 3.10-3.13 is required."
}

if (Test-Path -LiteralPath $venv) {
    $existingPython = Join-Path $venv 'Scripts\python.exe'
    $venvHealthy = (Test-Path -LiteralPath $existingPython)
    if ($venvHealthy) {
        & $existingPython -m pip --version *> $null
        $venvHealthy = ($LASTEXITCODE -eq 0)
    }
    if (-not $venvHealthy) {
        $resolvedProject = (Resolve-Path -LiteralPath $projectRoot).Path
        $resolvedVenv = (Resolve-Path -LiteralPath $venv).Path
        if ((Split-Path -Parent $resolvedVenv) -ne $resolvedProject -or (Split-Path -Leaf $resolvedVenv) -ne '.venv') {
            throw "Refusing to remove a virtual environment outside this project: $resolvedVenv"
        }
        Write-Host 'The existing .venv is incomplete. Rebuilding it safely.'
        Remove-Item -LiteralPath $resolvedVenv -Recurse -Force
    }
}

if (-not (Test-Path -LiteralPath $venv)) {
    & python -m venv $venv
    if ($LASTEXITCODE -ne 0) { throw 'Failed to create the Python virtual environment.' }
}

$python = Join-Path $venv 'Scripts\python.exe'
& $python -m pip install --disable-pip-version-check --quiet --upgrade pip
if ($LASTEXITCODE -ne 0) { throw 'Failed to upgrade pip.' }
& $python -m pip install --disable-pip-version-check --quiet -r $requirements
if ($LASTEXITCODE -ne 0) { throw 'Failed to install dependencies.' }

if (-not (Test-Path -LiteralPath $envFile)) {
    Copy-Item -LiteralPath $envExample -Destination $envFile
    Write-Host "Created $envFile"
}

Write-Host ''
Write-Host 'Installation completed. Next steps:'
Write-Host '1. Edit .env and set a new UPSTREAM_API_KEY.'
Write-Host '2. Run the gateway launcher in the project root.'
Write-Host '3. Run the Codex configuration launcher, then restart Codex.'
