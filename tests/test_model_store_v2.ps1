$ErrorActionPreference = 'Stop'

$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $projectRoot 'scripts\model-store.ps1')

$testRoot = Join-Path ([IO.Path]::GetTempPath()) ("ccg-model-store-v2-" + [guid]::NewGuid().ToString('N'))
try {
    $gatewayDir = Join-Path $testRoot '.gateway'
    New-Item -ItemType Directory -Path $gatewayDir -Force | Out-Null
    $oldJson = @'
{
  "version": 1,
  "default_id": "old-account",
  "profiles": [
    {
      "id": "old-account",
      "name": "Old account",
      "base_url": "https://example.invalid/v1",
      "api_key": "test-key",
      "model_id": "gpt-test",
      "litellm_model": "openai/gpt-test"
    }
  ]
}
'@
    [IO.File]::WriteAllText((Join-Path $gatewayDir 'models.json'), $oldJson, [Text.UTF8Encoding]::new($false))

    $store = Read-ModelStore -ProjectRoot $testRoot
    if ($store.version -ne 3) { throw 'v1 store was not upgraded in memory' }
    if ($store.routing.enabled) { throw 'routing must remain opt-in after migration' }
    if ($store.routing.affinity_ttl_seconds -ne 3600) { throw 'unexpected affinity TTL' }
    if (-not $store.profiles[0].routing_enabled) { throw 'old profile should participate by default' }
    if ($store.profiles[0].routing_weight -ne 1) { throw 'unexpected default routing weight' }
    if (@($store.routing.model_rules).Count -ne 0) { throw 'disabled v1 store should have no model rules' }

    $store.routing.enabled = $true
    $store.routing.model_rules = @([pscustomobject]@{ model_id = 'gpt-test'; enabled = $true })
    Save-ModelStore -ProjectRoot $testRoot -Store $store
    $saved = Get-Content -LiteralPath (Join-Path $gatewayDir 'models.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if (-not $saved.routing.enabled) { throw 'routing setting was not preserved' }
    if (-not $saved.routing.model_rules[0].enabled) { throw 'per-model routing rule was not preserved' }
    if ($saved.profiles[0].routing_weight -ne 1) { throw 'profile routing fields were not preserved' }

    Write-Host 'MODEL_STORE_V3_OK'
} finally {
    if (Test-Path -LiteralPath $testRoot) {
        [IO.Directory]::Delete($testRoot, $true)
    }
}
