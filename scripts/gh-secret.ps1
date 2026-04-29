$env_file = Join-Path $PSScriptRoot ".." ".env"
$vars = Get-Content $env_file | Where-Object { $_ -match "^\s*[^#].*=.*" } | ForEach-Object {
    $parts = $_ -split "=", 2
    [PSCustomObject]@{ Key = $parts[0].Trim(); Value = $parts[1].Trim() }
}

$secrets = @("GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET", "AZURE_CLIENT_ID", "NOTION_CLIENT_ID", "NOTION_CLIENT_SECRET")

foreach ($secret in $secrets) {
    $entry = $vars | Where-Object { $_.Key -eq $secret }
    if ($entry -and $entry.Value -ne "") {
        Write-Host "Setting $secret..."
        $entry.Value | gh secret set $secret --repo guidonaselli/skeepy-notes
    } else {
        Write-Host "SKIPPED $secret (not found or empty in .env)"
    }
}

Write-Host ""
Write-Host "Done. Current secrets:"
gh secret list --repo guidonaselli/skeepy-notes
