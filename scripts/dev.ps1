# =====================================================================
# dev.ps1 - Arranca el entorno de desarrollo de PokemonCardDetector.
# Abre tres ventanas: API Rust (:8787), servicio ML (:8001) y web (:5173).
# Compatible con Windows PowerShell 5.1 (sin operadores && ni ||).
# Uso:  .\scripts\dev.ps1
# =====================================================================

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$apiDir = Join-Path $repoRoot "services\api"
$mlDir  = Join-Path $repoRoot "services\ml"
$webDir = Join-Path $repoRoot "apps\web"

# --- Comprobacion de prerequisitos ---------------------------------
function Test-Tool {
    param(
        [string]$Name,
        [string]$Hint
    )
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($null -eq $cmd) {
        Write-Host "[FALTA] No se encontro '$Name'. $Hint" -ForegroundColor Red
        return $false
    }
    Write-Host "[OK] $Name -> $($cmd.Source)" -ForegroundColor Green
    return $true
}

Write-Host "Comprobando prerequisitos..." -ForegroundColor Cyan
$ok = $true
if (-not (Test-Tool -Name "cargo"  -Hint "Instala Rust desde https://rustup.rs"))            { $ok = $false }
if (-not (Test-Tool -Name "python" -Hint "Instala Python 3.10+ desde https://python.org"))   { $ok = $false }
if (-not (Test-Tool -Name "npm"    -Hint "Instala Node.js 18+ desde https://nodejs.org"))    { $ok = $false }
if (-not $ok) {
    Write-Host "Faltan prerequisitos. Instalalos y vuelve a ejecutar el script." -ForegroundColor Red
    exit 1
}

# --- API Rust (:8787) ----------------------------------------------
Write-Host "Arrancando API Rust en http://127.0.0.1:8787 ..." -ForegroundColor Cyan
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cargo run" -WorkingDirectory $apiDir

# --- Servicio ML (:8001) -------------------------------------------
# Usa el venv de services/ml si existe; si no, el python global.
$venvPython = Join-Path $mlDir ".venv\Scripts\python.exe"
if (Test-Path $venvPython) {
    $mlCommand = "& `"$venvPython`" -m uvicorn app.main:app --port 8001"
    Write-Host "Arrancando servicio ML (venv) en http://127.0.0.1:8001 ..." -ForegroundColor Cyan
} else {
    & python -c "import uvicorn"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[FALTA] El python global no puede importar 'uvicorn'." -ForegroundColor Red
        Write-Host "Crea el venv e instala las dependencias en services\ml:" -ForegroundColor Red
        Write-Host "  python -m venv .venv; .\.venv\Scripts\Activate.ps1; pip install -r requirements.txt" -ForegroundColor Red
        exit 1
    }
    $mlCommand = "python -m uvicorn app.main:app --port 8001"
    Write-Host "Arrancando servicio ML (python global, no se encontro .venv) en http://127.0.0.1:8001 ..." -ForegroundColor Yellow
}
Start-Process powershell -ArgumentList "-NoExit", "-Command", $mlCommand -WorkingDirectory $mlDir

# --- Web (:5173) ----------------------------------------------------
if (-not (Test-Path (Join-Path $webDir "node_modules"))) {
    Write-Host "[FALTA] No existe apps\web\node_modules. Ejecuta 'npm install' en apps\web y vuelve a lanzar el script." -ForegroundColor Red
    exit 1
}
Write-Host "Arrancando web en http://localhost:5173 ..." -ForegroundColor Cyan
Start-Process powershell -ArgumentList "-NoExit", "-Command", "npm run dev" -WorkingDirectory $webDir

Write-Host ""
Write-Host "Todo lanzado. Ventanas abiertas:" -ForegroundColor Green
Write-Host "  - API Rust:    http://127.0.0.1:8787"
Write-Host "  - Servicio ML: http://127.0.0.1:8001"
Write-Host "  - Web:         http://localhost:5173"
Write-Host "Recuerda: si es la primera vez, ejecuta antes la ingesta (scripts\ingest.ps1)."
