# CardLens — Shell de escritorio y Android (Tauri 2)

Esta carpeta contiene el shell nativo construido con [Tauri 2](https://v2.tauri.app/).
No tiene interfaz propia: en desarrollo carga la web servida por Vite
(`http://localhost:5173`) y en produccion empaqueta el build estatico de
`apps/web` (`../../web/dist` relativo a `src-tauri`).

## Requisitos

- **Rust** (toolchain estable, instalado con [rustup](https://rustup.rs/)).
- **Node.js** (18 o superior) y npm.
- En **Windows** no hace falta nada mas: el runtime **WebView2 viene incluido**
  en Windows 10/11.

Instala la CLI de Tauri (definida como devDependency):

```bash
npm install
```

## Flujo de desarrollo

Se necesitan dos terminales:

1. **Terminal 1** — arranca la web con Vite:

   ```bash
   cd apps/web
   npm run dev
   ```

2. **Terminal 2** — arranca el shell Tauri (esta carpeta):

   ```bash
   cd apps/desktop
   npm run dev
   ```

Tauri abrira una ventana nativa apuntando a `http://localhost:5173`
(`build.devUrl` en `src-tauri/tauri.conf.json`). Recuerda tener tambien en
marcha la API Rust (`http://127.0.0.1:8787`) y el servicio ML
(`http://127.0.0.1:8001`) para que la app funcione de verdad.

## Notas de compilacion (Windows)

Dos ajustes ya aplicados en este repo que conviene conocer:

- **Crate `time` fijada a 0.3.47** en `src-tauri/Cargo.lock`: la 0.3.48 no
  compila con rustc 1.95 (error E0119 en `cookie`/`tauri-utils`). Si un
  `cargo update` la vuelve a subir y aparece ese error, re-fija con
  `cargo update -p time --precise 0.3.47` (o actualiza la toolchain de Rust).
- **Iconos generados** en `src-tauri/icons/` (desde `apps/web/public/icon.svg`
  con `npx tauri icon`): en Windows `tauri-build` exige `icons/icon.ico`
  para el recurso del .exe aunque `bundle.active` sea `false`.

## Build de escritorio

El empaquetado de instaladores esta desactivado de momento
(`bundle.active: false` en `tauri.conf.json`).
Para generar un instalable:

1. Genera el build de la web: `cd apps/web && npm run build` (crea `apps/web/dist`).
2. Los iconos ya estan generados en `src-tauri/icons/`; si quieres otros,
   regeneralos a partir de un PNG o SVG cuadrado (1024x1024 recomendado):

   ```bash
   npx tauri icon ruta-a-icono.png
   ```

3. Activa el bundle: en `src-tauri/tauri.conf.json` pon `"bundle": { "active": true }`.
4. Ejecuta el build:

   ```bash
   npm run build
   ```

Los artefactos quedan en `src-tauri/target/release/bundle/`.

## Android

### Requisitos

- **Android Studio** con el **SDK** y el **NDK** instalados (SDK Manager →
  pestañas *SDK Platforms* y *SDK Tools* → marca *NDK (Side by side)*).
- **JDK 17**.
- Variables de entorno configuradas:
  - `ANDROID_HOME` → ruta del SDK (p. ej. `C:\Users\<usuario>\AppData\Local\Android\Sdk`).
  - `NDK_HOME` → ruta del NDK (p. ej. `%ANDROID_HOME%\ndk\<version>`).
- Targets de Rust para Android:

  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
  ```

### Inicializar el proyecto Android

```bash
npx tauri android init
```

Esto genera el proyecto nativo en `src-tauri/gen/android`. Despues de
inicializar, **añade los permisos de camara e internet** al manifiesto
generado (`src-tauri/gen/android/app/src/main/AndroidManifest.xml`):

```xml
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.CAMERA" />
```

> **Nota sobre la camara:** `getUserMedia` dentro del WebView de Android puede
> requerir ademas conceder manualmente el permiso de camara a la app
> (Ajustes → Aplicaciones → CardLens → Permisos). Si la camara en
> vivo no funciona, el modo **Subir** de la web (input de fichero con
> `capture`) funciona siempre como fallback: abre la camara nativa del sistema.

### Ejecutar en un dispositivo o emulador

```bash
npx tauri android dev
```

## Nota sobre la red y la API

La app consume la API por red segun la variable `VITE_API_URL` con la que se
construyo la web (default `http://localhost:8787`). En **Android**,
`localhost` apunta al propio dispositivo, asi que al construir la web para el
movil debes usar la **IP LAN de la maquina que sirve la API**, por ejemplo:

```powershell
# en apps/web
$env:VITE_API_URL = 'http://192.168.1.50:8787'; npm run build
```

Alternativamente, puedes definir `VITE_API_URL=http://192.168.1.50:8787` en
`apps/web/.env` (partiendo de `apps/web/.env.example`) y ejecutar
`npm run build` sin mas.

(y asegurate de que la API Rust escucha en una interfaz accesible desde la red
local y de que el firewall permite el puerto 8787).
