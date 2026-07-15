$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'model-store.ps1')

function Read-SecretText {
    $secure = Read-Host 'API key (input is hidden)' -AsSecureString
    $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try { return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer) }
    finally { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer) }
}

function Get-MaskedKey([string]$Key) {
    if ($Key.Length -le 8) { return '********' }
    return $Key.Substring(0, 3) + '...' + $Key.Substring($Key.Length - 4)
}

function Show-Profiles {
    $store = Read-ModelStore -ProjectRoot $projectRoot
    Write-Host ''
    if (@($store.profiles).Count -eq 0) { Write-Host 'No model configurations.'; return }
    $number = 1
    foreach ($profile in @($store.profiles)) {
        $mark = if ($profile.id -eq $store.default_id) { '*' } else { ' ' }
        Write-Host ("{0} [{1}] {2} | {3} | {4} | {5}" -f $number, $mark, $profile.name, $profile.base_url, $profile.litellm_model, (Get-MaskedKey $profile.api_key))
        $number++
    }
    Write-Host '* = current default'
}

function Select-Profile([string]$Prompt) {
    $store = Read-ModelStore -ProjectRoot $projectRoot
    $profiles = @($store.profiles)
    if ($profiles.Count -eq 0) { Write-Host 'No model configurations.'; return $null }
    Show-Profiles
    $answer = Read-Host $Prompt
    $index = 0
    if (-not [int]::TryParse($answer, [ref]$index) -or $index -lt 1 -or $index -gt $profiles.Count) {
        Write-Host 'Invalid selection.'
        return $null
    }
    return [pscustomobject]@{ store = $store; profile = $profiles[$index - 1] }
}

function Add-Profile {
    Write-Host ''
    $baseUrl = (Read-Host 'API base URL (usually ends with /v1)').Trim().TrimEnd('/')
    if (-not [uri]::IsWellFormedUriString($baseUrl, [UriKind]::Absolute)) { Write-Host 'Invalid URL.'; return }
    $apiKey = Read-SecretText
    if (-not $apiKey) { Write-Host 'API key cannot be empty.'; return }

    Write-Host '1. Manual model'
    Write-Host '2. Browse models from the API'
    $mode = Read-Host 'Select model mode [2]'
    if (-not $mode) { $mode = '2' }
    $modelId = ''
    if ($mode -eq '1') {
        $modelId = (Read-Host 'Model ID').Trim()
    } elseif ($mode -eq '2') {
        try {
            $headers = @{ Authorization = "Bearer $apiKey" }
            $response = Invoke-RestMethod -Method Get -Uri "$baseUrl/models" -Headers $headers -TimeoutSec 30
            $models = @($response.data | ForEach-Object { [string]$_.id } | Where-Object { $_ } | Sort-Object -Unique)
            if ($models.Count -eq 0) { Write-Host 'The API returned no models.'; return }
            for ($i = 0; $i -lt $models.Count; $i++) { Write-Host ("{0}. {1}" -f ($i + 1), $models[$i]) }
            $choice = Read-Host 'Select model number'
            $selected = 0
            if (-not [int]::TryParse($choice, [ref]$selected) -or $selected -lt 1 -or $selected -gt $models.Count) { Write-Host 'Invalid selection.'; return }
            $modelId = $models[$selected - 1]
        } catch {
            Write-Host "Could not browse models: $($_.Exception.Message)"
            Write-Host 'Nothing was saved. Retry and choose Manual model if this provider does not expose /models.'
            return
        }
    } else { Write-Host 'Invalid selection.'; return }

    if (-not $modelId) { Write-Host 'Model ID cannot be empty.'; return }
    $defaultName = $modelId
    $name = (Read-Host "Configuration name [$defaultName]").Trim()
    if (-not $name) { $name = $defaultName }
    $litellmModel = Get-LiteLLMModelName -BaseUrl $baseUrl -ModelId $modelId
    $store = Read-ModelStore -ProjectRoot $projectRoot
    $id = [guid]::NewGuid().ToString('N')
    $profile = [pscustomobject]@{ id = $id; name = $name; base_url = $baseUrl; api_key = $apiKey; model_id = $modelId; litellm_model = $litellmModel }
    $profiles = @($store.profiles) + @($profile)
    $defaultId = if ($store.default_id) { $store.default_id } else { $id }
    Save-ModelStore -ProjectRoot $projectRoot -Store ([pscustomobject]@{ version = 1; default_id = $defaultId; profiles = $profiles })
    $apiKey = $null
    Write-Host "Saved: $name ($litellmModel)"
}

function Delete-Profile {
    $selection = Select-Profile 'Delete model number'
    if (-not $selection) { return }
    $store = $selection.store
    $profile = $selection.profile
    $profiles = @($store.profiles | Where-Object { $_.id -ne $profile.id })
    $defaultId = $store.default_id
    if ($defaultId -eq $profile.id) { $defaultId = if ($profiles.Count -gt 0) { $profiles[0].id } else { '' } }
    Save-ModelStore -ProjectRoot $projectRoot -Store ([pscustomobject]@{ version = 1; default_id = $defaultId; profiles = $profiles })
    Write-Host "Deleted: $($profile.name)"
}

function Set-DefaultProfile {
    $selection = Select-Profile 'Default model number'
    if (-not $selection) { return }
    $selection.store.default_id = $selection.profile.id
    Save-ModelStore -ProjectRoot $projectRoot -Store $selection.store
    Write-Host "Current default: $($selection.profile.name)"
    Write-Host 'Restart the gateway to apply the new default.'
}

if (Import-LegacyEnvironment -ProjectRoot $projectRoot) { Write-Host 'Imported the existing .env as the default model.' }
do {
    Show-Profiles
    Write-Host ''
    Write-Host '1. Add model configuration'
    Write-Host '2. Delete model configuration'
    Write-Host '3. Set current default model'
    Write-Host '4. Exit'
    $action = Read-Host 'Select action'
    switch ($action) {
        '1' { Add-Profile }
        '2' { Delete-Profile }
        '3' { Set-DefaultProfile }
        '4' { break }
        default { Write-Host 'Invalid selection.' }
    }
} while ($action -ne '4')
