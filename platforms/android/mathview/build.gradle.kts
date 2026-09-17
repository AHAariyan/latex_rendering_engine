plugins {
  alias(libs.plugins.android.library)
  alias(libs.plugins.compose.compiler)
}

android {
  namespace = "dev.mathcore"
  compileSdk = 36
  defaultConfig {
    minSdk = 24
    ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
  }
  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
  buildFeatures { compose = true }
  sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
}

kotlin { jvmToolchain(17) }

// Cross-compiles the Rust engine into src/main/jniLibs before every build.
// Skip with -PskipCargo when the .so files are already present.
val repoRoot: File = rootProject.projectDir.resolve("../..")
val jniLibsDir: File = projectDir.resolve("src/main/jniLibs")
val cargoNdk = tasks.register<Exec>("cargoNdk") {
  workingDir = repoRoot
  commandLine(repoRoot.resolve("scripts/build-android.sh").absolutePath, jniLibsDir.absolutePath)
}
if (!providers.gradleProperty("skipCargo").isPresent) {
  tasks.named("preBuild") { dependsOn(cargoNdk) }
}

dependencies {
  val composeBom = platform(libs.androidx.compose.bom)
  implementation(composeBom)
  implementation(libs.androidx.compose.ui)
  implementation(libs.androidx.compose.foundation)
}
