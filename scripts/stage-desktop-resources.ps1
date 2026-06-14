# Prepara el binario sidecar y los recursos que Tauri empaqueta en el .exe de
# escritorio. Ejecutar antes de `npm run tauri build` (o de cargo check del
# shell con bundle activo). Copia desde las salidas de build/descargas a:
#   apps/desktop/src-tauri/binaries/   (el binario de la API con sufijo de target)
#   apps/desktop/src-tauri/resources/  (modelo, indice, OCR, DLL de ONNX, DB)
# Estas carpetas estan ignoradas por git (artefactos grandes).

$ErrorActionPreference = "Stop"
$root = "E:\projects\PokemonCardDetector"
$triple = "x86_64-pc-windows-msvc"
$bin = Join-Path $root "apps\desktop\src-tauri\binaries"
$res = Join-Path $root "apps\desktop\src-tauri\resources"

New-Item -ItemType Directory -Force $bin | Out-Null
New-Item -ItemType Directory -Force (Join-Path $res "mobileclip2_s0") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $res "index") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $res "ocrs") | Out-Null

$apiExe = Join-Path $root "services\api\target\release\pokemon-card-api.exe"
if (-not (Test-Path $apiExe)) { throw "Falta la API release: compila con 'cargo build --release' en services\api" }
Copy-Item $apiExe (Join-Path $bin "cardlens-api-$triple.exe") -Force

$pairs = @(
  @{ src = "runtime\ort\onnxruntime.dll";                 dst = "onnxruntime.dll" },
  @{ src = "data\app.db";                                 dst = "app.db" },
  @{ src = "models\mobileclip2_s0\vision_model.onnx";     dst = "mobileclip2_s0\vision_model.onnx" },
  @{ src = "data\index\mobileclip.bin";                   dst = "index\mobileclip.bin" },
  @{ src = "data\index\mobileclip_cards.json";            dst = "index\mobileclip_cards.json" },
  @{ src = "models\ocrs\text-detection.rten";             dst = "ocrs\text-detection.rten" },
  @{ src = "models\ocrs\text-recognition.rten";           dst = "ocrs\text-recognition.rten" }
)
foreach ($p in $pairs) {
  $s = Join-Path $root $p.src
  if (-not (Test-Path $s)) { throw "Falta el recurso: $($p.src)" }
  Copy-Item $s (Join-Path $res $p.dst) -Force
}
Write-Output "Recursos de escritorio preparados en binaries\ y resources\"
