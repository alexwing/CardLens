import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

// Firma de release: lee gen/android/keystore.properties (ignorado por git). Si
// no existe (otra maquina sin el keystore), el build de release no se firma.
val keystoreProperties = Properties().apply {
    val propFile = rootProject.file("keystore.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    namespace = "xyz.mappuzzle.cardlens"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "xyz.mappuzzle.cardlens"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    // Los modelos/indice/DB van como assets y se leen con AssetManager, que NO
    // puede abrir assets comprimidos mayores de ~1 MB. Hay que dejarlos sin
    // comprimir (ademas evita recomprimir ~138 MB ya densos).
    androidResources {
        noCompress += listOf("onnx", "bin", "rten", "db", "json")
    }
    signingConfigs {
        create("release") {
            keystoreProperties.getProperty("keyAlias")?.let { keyAlias = it }
            keystoreProperties.getProperty("keyPassword")?.let { keyPassword = it }
            keystoreProperties.getProperty("storePassword")?.let { storePassword = it }
            keystoreProperties.getProperty("storeFile")?.let { storeFile = file(it) }
        }
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            // Firma con el keystore de release si esta configurado.
            if (keystoreProperties.containsKey("storeFile")) {
                signingConfig = signingConfigs.getByName("release")
            }
            // Sin minify/proguard: el peso lo dominan los assets (~138 MB) y la
            // .so de Rust; R8 sobre el poco Java/Kotlin no compensa el riesgo de
            // recortar algo que Tauri/wry necesiten en la primera release.
            isMinifyEnabled = false
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")