# Prepara los recursos nativos y pesados que la APK de Android empaqueta.
# Ejecutar antes de `npm run tauri android build` (o `android dev`). Copia a:
#   gen/android/app/src/main/jniLibs/<abi>/libonnxruntime.so   (ONNX Runtime nativo)
#   gen/android/app/src/main/assets/cardlens/...               (modelo, indice, OCR, DB)
# Ambas rutas estan bajo gen/android (ignorado por git): son artefactos grandes.
#
# Uso: powershell -File scripts/stage-android-resources.ps1 [-Abis arm64-v8a,x86_64]
param(
  [string[]] $Abis = @('arm64-v8a')
)

$ErrorActionPreference = "Stop"
$root = "E:\projects\PokemonCardDetector"
$android = Join-Path $root "apps\desktop\src-tauri\gen\android\app\src\main"
$jniRoot = Join-Path $android "jniLibs"
$assets = Join-Path $android "assets\cardlens"

if (-not (Test-Path (Join-Path $root "apps\desktop\src-tauri\gen\android"))) {
  throw "No existe gen/android. Ejecuta antes: cd apps/desktop; npm run tauri -- android init"
}

# 1) libonnxruntime.so por ABI (desde runtime/ort/android/<abi>/, extraido del AAR oficial).
foreach ($abi in $Abis) {
  $so = Join-Path $root "runtime\ort\android\$abi\libonnxruntime.so"
  if (-not (Test-Path $so)) { throw "Falta $so (extrae el AAR onnxruntime-android)" }
  $dst = Join-Path $jniRoot $abi
  New-Item -ItemType Directory -Force $dst | Out-Null
  Copy-Item $so (Join-Path $dst "libonnxruntime.so") -Force
}

# 2) Recursos pesados como assets (se copian a datos en el primer arranque).
New-Item -ItemType Directory -Force (Join-Path $assets "mobileclip2_s0") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $assets "index") | Out-Null
New-Item -ItemType Directory -Force (Join-Path $assets "ocrs") | Out-Null

$pairs = @(
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
  Copy-Item $s (Join-Path $assets $p.dst) -Force
}

Write-Output ("Recursos Android preparados: jniLibs ({0}) + assets/cardlens" -f ($Abis -join ", "))
