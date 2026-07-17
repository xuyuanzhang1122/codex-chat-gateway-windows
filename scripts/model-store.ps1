$ErrorActionPreference = 'Stop'

function Get-ModelStorePath {
    param([string]$ProjectRoot)
    return (Join-Path $ProjectRoot '.gateway\models.json')
}

function Read-ModelStore {
    param([string]$ProjectRoot)
    $path = Get-ModelStorePath $ProjectRoot
    if (-not (Test-Path -LiteralPath $path)) {
        return [pscustomobject]@{ version = 1; default_id = ''; profiles = @() }
    }
    $store = Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $store.profiles) { $store.profiles = @() }
    return $store
}

function Save-ModelStore {
    param([string]$ProjectRoot, [object]$Store)
    $path = Get-ModelStorePath $ProjectRoot
    $directory = Split-Path -Parent $path
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    $temporary = "$path.tmp"
    $json = $Store | ConvertTo-Json -Depth 8
    [IO.File]::WriteAllText($temporary, $json, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $path -Force
}

function Get-LiteLLMModelName {
    param([string]$BaseUrl, [string]$ModelId)
    if ($BaseUrl -match '(?i)deepseek') {
        if ($ModelId -match '^deepseek/') { return $ModelId }
        return "deepseek/$ModelId"
    }
    if ($ModelId -match '^openai/') { return $ModelId }
    return "openai/$ModelId"
}

function Get-ClaudeLiteLLMModelName {
    param([string]$LiteLLMModel)
    if ($LiteLLMModel -match '^openai/(.+)$') { return "custom_openai/$($matches[1])" }
    return $LiteLLMModel
}

function Import-LegacyEnvironment {
    param([string]$ProjectRoot)
    $storePath = Get-ModelStorePath $ProjectRoot
    $envPath = Join-Path $ProjectRoot '.env'
    if ((Test-Path -LiteralPath $storePath) -or -not (Test-Path -LiteralPath $envPath)) { return $false }

    $values = @{}
    foreach ($line in Get-Content -LiteralPath $envPath -Encoding UTF8) {
        if ($line -match '^\s*([^#=]+?)\s*=\s*(.*)$') { $values[$matches[1]] = $matches[2] }
    }
    if (-not $values.UPSTREAM_MODEL -or -not $values.UPSTREAM_BASE_URL -or -not $values.UPSTREAM_API_KEY) { return $false }
    if ($values.UPSTREAM_API_KEY -eq 'replace-with-new-key') { return $false }

    $id = [guid]::NewGuid().ToString('N')
    $profile = [pscustomobject]@{
        id = $id
        name = 'Imported model'
        base_url = $values.UPSTREAM_BASE_URL
        api_key = $values.UPSTREAM_API_KEY
        model_id = ($values.UPSTREAM_MODEL -replace '^[^/]+/', '')
        litellm_model = $values.UPSTREAM_MODEL
    }
    $store = [pscustomobject]@{ version = 1; default_id = $id; profiles = @($profile) }
    Save-ModelStore -ProjectRoot $ProjectRoot -Store $store
    return $true
}

function Set-DefaultModelEnvironment {
    param([string]$ProjectRoot)
    Import-LegacyEnvironment -ProjectRoot $ProjectRoot | Out-Null
    $store = Read-ModelStore -ProjectRoot $ProjectRoot
    $profile = @($store.profiles | Where-Object { $_.id -eq $store.default_id }) | Select-Object -First 1
    if (-not $profile) { throw 'No default model is configured. Run model-config.bat first.' }
    $env:UPSTREAM_MODEL = [string]$profile.litellm_model
    $env:CLAUDE_UPSTREAM_MODEL = Get-ClaudeLiteLLMModelName -LiteLLMModel ([string]$profile.litellm_model)
    $env:UPSTREAM_BASE_URL = [string]$profile.base_url
    $env:UPSTREAM_API_KEY = [string]$profile.api_key
    $env:GATEWAY_HOST = '127.0.0.1'
    $env:GATEWAY_PORT = '4000'
    return $profile
}
