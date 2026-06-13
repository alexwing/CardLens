# =====================================================================
# ingest.ps1 - Ingesta del catalogo y construccion del indice FAISS.
# 1) python -m ingest.ingest_catalog <argumentos recibidos>
# 2) python -m ingest.build_index
# Compatible con Windows PowerShell 5.1 (sin operadores && ni ||).
# Uso:  .\scripts\ingest.ps1 --langs en es --sets base1 swsh3
#       .\scripts\ingest.ps1 --all
# =====================================================================

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$mlDir = Join-Path $repoRoot "services\ml"

if (-not (Test-Path $mlDir)) {
    Write-Host "[ERROR] No se encontro el directorio $mlDir" -ForegroundColor Red
    exit 1
}

# Usa el python del venv de services/ml si existe; si no, el global.
$venvPython = Join-Path $mlDir ".venv\Scripts\python.exe"
if (Test-Path $venvPython) {
    $python = $venvPython
    Write-Host "[OK] Usando el venv: $venvPython" -ForegroundColor Green
} else {
    $cmd = Get-Command python -ErrorAction SilentlyContinue
    if ($null -eq $cmd) {
        Write-Host "[ERROR] No se encontro 'python'. Instala Python 3.10+ o crea el venv en services\ml\.venv" -ForegroundColor Red
        exit 1
    }
    $python = "python"
    Write-Host "[AVISO] No existe services\ml\.venv; se usara el python global." -ForegroundColor Yellow
}

Push-Location $mlDir
try {
    Write-Host "Paso 1/2: ingesta del catalogo (python -m ingest.ingest_catalog $args)..." -ForegroundColor Cyan
    & $python -m ingest.ingest_catalog @args
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] La ingesta del catalogo fallo (codigo $LASTEXITCODE). Se aborta." -ForegroundColor Red
        exit $LASTEXITCODE
    }

    Write-Host "Paso 2/2: construccion del indice FAISS (python -m ingest.build_index)..." -ForegroundColor Cyan
    & $python -m ingest.build_index
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] La construccion del indice fallo (codigo $LASTEXITCODE)." -ForegroundColor Red
        exit $LASTEXITCODE
    }

    Write-Host "Ingesta completada: catalogo en data\app.db e indice en data\index\faiss.index" -ForegroundColor Green
} finally {
    Pop-Location
}
