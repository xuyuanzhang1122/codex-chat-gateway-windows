$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$envFile = Join-Path $projectRoot '.env'

$defaultModel = 'deepseek/deepseek-chat'
$defaultBaseUrl = 'https://api.deepseek.com'
$defaultHost = '127.0.0.1'
$defaultPort = '4000'

Write-Host 'First-run gateway configuration'
$model = Read-Host "Upstream model [$defaultModel]"
if (-not $model) { $model = $defaultModel }
$baseUrl = Read-Host "Upstream base URL [$defaultBaseUrl]"
if (-not $baseUrl) { $baseUrl = $defaultBaseUrl }
$secureKey = Read-Host 'New upstream API key (input is hidden)' -AsSecureString
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secureKey)
try {
    $apiKey = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
} finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
}

if (-not $apiKey) { throw 'API key cannot be empty.' }
if ($apiKey.Contains("`r") -or $apiKey.Contains("`n")) { throw 'API key cannot contain line breaks.' }
if ($model.Contains("`r") -or $model.Contains("`n")) { throw 'Model cannot contain line breaks.' }
if ($baseUrl.Contains("`r") -or $baseUrl.Contains("`n")) { throw 'Base URL cannot contain line breaks.' }

$lines = @(
    "UPSTREAM_MODEL=$model",
    "UPSTREAM_BASE_URL=$baseUrl",
    "UPSTREAM_API_KEY=$apiKey",
    "GATEWAY_HOST=$defaultHost",
    "GATEWAY_PORT=$defaultPort"
)
[IO.File]::WriteAllLines($envFile, $lines, [Text.UTF8Encoding]::new($false))
$apiKey = $null
Write-Host "Gateway settings saved to $envFile"

