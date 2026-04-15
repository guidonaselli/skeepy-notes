# setup-release.ps1
# Run this script once before publishing the first release.
# Usage (from repo root):  pwsh scripts\setup-release.ps1

param(
    [string]$GithubUser = "",
    [string]$GithubRepo = "skeepy-notes"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Step { param([string]$msg) Write-Host "`n>>> $msg" -ForegroundColor Cyan }
function Write-Ok   { param([string]$msg) Write-Host "  OK  $msg" -ForegroundColor Green }
function Write-Warn { param([string]$msg) Write-Host "  WARN $msg" -ForegroundColor Yellow }
function Write-Err  { param([string]$msg) Write-Host "  ERR  $msg" -ForegroundColor Red }

# ─── 0. Collect GitHub username ───────────────────────────────────────────────
if (-not $GithubUser) {
    $GithubUser = Read-Host "  Enter your GitHub username"
}
if (-not $GithubUser) {
    Write-Err "GitHub username is required. Run: pwsh scripts\setup-release.ps1 -GithubUser YOUR_USERNAME"
    exit 1
}

Write-Host ""
Write-Host "=== Skeepy Release Setup ===" -ForegroundColor White
Write-Host "  GitHub user : $GithubUser"
Write-Host "  GitHub repo : $GithubRepo"

# ─── 1. Check prerequisites ───────────────────────────────────────────────────
Write-Step "Checking prerequisites"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Err "cargo not found. Install Rust from https://rustup.rs"
    exit 1
}
Write-Ok "Rust / cargo found"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
    Write-Warn "GitHub CLI (gh) not found — install from https://cli.github.com if you want automated repo creation"
} else {
    Write-Ok "GitHub CLI found"
}

# ─── 2. Generate minisign signing keypair ────────────────────────────────────
Write-Step "Generating minisign signing keypair"

$KeyDir = Join-Path $HOME ".tauri"
$KeyFile = Join-Path $KeyDir "skeepy.key"

if (Test-Path $KeyFile) {
    Write-Warn "Key already exists at $KeyFile — skipping generation"
    Write-Warn "Delete it first if you want to regenerate."
} else {
    New-Item -ItemType Directory -Force -Path $KeyDir | Out-Null
    Write-Host "  Running: cargo tauri signer generate -w $KeyFile"
    & cargo tauri signer generate -w $KeyFile
    Write-Ok "Keys generated at $KeyFile"
}

# ─── 3. Read public key ───────────────────────────────────────────────────────
Write-Step "Reading public key"

$PubKeyFile = "$KeyFile.pub"
if (-not (Test-Path $PubKeyFile)) {
    Write-Err "Public key not found at $PubKeyFile"
    exit 1
}

$PubKeyContent = Get-Content $PubKeyFile -Raw
# The .pub file has two lines: a comment and the actual key
$PubKeyLines = ($PubKeyContent -split "`n") | Where-Object { $_ -notmatch "^untrusted comment:" }
$PubKey = ($PubKeyLines | Select-Object -First 1).Trim()

Write-Ok "Public key: $PubKey"

# ─── 4. Patch tauri.conf.json ─────────────────────────────────────────────────
Write-Step "Patching src-tauri/tauri.conf.json"

$ConfPath = Join-Path $PSScriptRoot ".." "src-tauri" "tauri.conf.json"
$ConfPath = [System.IO.Path]::GetFullPath($ConfPath)

if (-not (Test-Path $ConfPath)) {
    Write-Err "tauri.conf.json not found at $ConfPath"
    exit 1
}

$Conf = Get-Content $ConfPath -Raw

$UpdatedEndpoint = "https://github.com/$GithubUser/$GithubRepo/releases/latest/download/latest.json"

$Conf = $Conf -replace '"REPLACE_WITH_YOUR_PUBLIC_KEY"', """$PubKey"""
$Conf = $Conf -replace '"https://github\.com/YOUR_GITHUB_USERNAME/skeepy-notes/releases/latest/download/latest\.json"', """$UpdatedEndpoint"""

Set-Content -Path $ConfPath -Value $Conf -Encoding UTF8
Write-Ok "tauri.conf.json patched with pubkey and endpoint"

# ─── 5. Display private key for GitHub secrets ────────────────────────────────
Write-Step "Private key for GitHub Secrets"

$PrivKeyContent = Get-Content $KeyFile -Raw

Write-Host ""
Write-Host "  Add the following as GitHub Action Secrets:" -ForegroundColor Yellow
Write-Host "  (Settings > Secrets and variables > Actions > New repository secret)" -ForegroundColor Yellow
Write-Host ""
Write-Host "  Secret name : TAURI_SIGNING_PRIVATE_KEY" -ForegroundColor White
Write-Host "  Secret value:" -ForegroundColor White
Write-Host $PrivKeyContent -ForegroundColor Gray
Write-Host ""
Write-Host "  Secret name : TAURI_SIGNING_PRIVATE_KEY_PASSWORD" -ForegroundColor White
Write-Host "  Secret value: (the password you entered when generating the key, or empty)" -ForegroundColor White

# ─── 6. Optionally create GitHub repo ────────────────────────────────────────
if (Get-Command gh -ErrorAction SilentlyContinue) {
    Write-Step "GitHub repository"
    $AuthStatus = & gh auth status 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Not authenticated with GitHub CLI. Run 'gh auth login' first."
    } else {
        $Confirm = Read-Host "  Create GitHub repo '$GithubUser/$GithubRepo' now? [y/N]"
        if ($Confirm -eq "y" -or $Confirm -eq "Y") {
            & gh repo create "$GithubUser/$GithubRepo" --public --description "Skeepy Notes — local-first sticky notes aggregator"
            Write-Ok "Repository created: https://github.com/$GithubUser/$GithubRepo"

            & git remote add origin "https://github.com/$GithubUser/$GithubRepo.git" 2>$null
            Write-Ok "Remote origin set"
        }
    }
}

# ─── 7. Summary ───────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor White
Write-Host "  1. Add TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD" -ForegroundColor Yellow
Write-Host "     to GitHub repository secrets (see above)" -ForegroundColor Yellow
Write-Host "  2. Also add GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET, AZURE_CLIENT_ID," -ForegroundColor Yellow
Write-Host "     NOTION_CLIENT_ID / NOTION_CLIENT_SECRET (from your OAuth apps)" -ForegroundColor Yellow
Write-Host "  3. Commit and push:" -ForegroundColor Yellow
Write-Host "       git add -A && git commit -m 'chore: configure release signing'" -ForegroundColor Gray
Write-Host "       git push -u origin main" -ForegroundColor Gray
Write-Host "  4. Tag and push to trigger the release workflow:" -ForegroundColor Yellow
Write-Host "       git tag v0.1.0 && git push origin v0.1.0" -ForegroundColor Gray
Write-Host ""
Write-Host "The GitHub Actions workflow will build the installers, sign them," -ForegroundColor White
Write-Host "generate latest.json, and publish the release automatically." -ForegroundColor White
