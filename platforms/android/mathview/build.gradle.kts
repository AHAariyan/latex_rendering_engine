plugins {
  alias(libs.plugins.android.library)
  alias(libs.plugins.compose.compiler)
  `maven-publish`
}

group = "dev.mathcore"
version = providers.gradleProperty("mathcoreVersion").getOrElse("0.0.0-SNAPSHOT")

android {
  namespace = "dev.mathcore"
  compileSdk = 36
  defaultConfig {
    minSdk = 24
    ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    consumerProguardFiles("consumer-rules.pro")
    testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
  }
  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
  buildFeatures { compose = true }
  sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
  testOptions { unitTests.isReturnDefaultValues = true }
  publishing {
    singleVariant("release") { withSourcesJar() }
  }
}

// dev.mathcore:mathcore-android, published by the SDK pipeline to a local
// Maven repository (-PdistRepo) that a release uploads to Maven Central.
afterEvaluate {
  publishing {
    publications {
      create<MavenPublication>("release") {
        from(components["release"])
        artifactId = "mathcore-android"
        pom {
          name.set("mathcore for Android")
          description.set("Native TeX math typesetting: Compose MathText, MathView, TalkBack speech and part navigation.")
          url.set("https://github.com/AHAariyan/latex_rendering_engine")
          licenses { license { name.set("MIT"); url.set("https://opensource.org/licenses/MIT") } }
          scm { url.set("https://github.com/AHAariyan/latex_rendering_engine") }
          developers { developer { id.set("mathcore"); name.set("mathcore") } }
        }
      }
    }
    repositories {
      maven {
        name = "dist"
        url = uri(providers.gradleProperty("distRepo").getOrElse(layout.buildDirectory.dir("repo").get().asFile.path))
      }
    }
  }
}

// JVM unit tests load a host build of the engine (cargo xtask sdk android
// passes -PhostLibDir); without it they are skipped.
val hostLibDir: String? = providers.gradleProperty("hostLibDir").orNull
tasks.withType<Test>().configureEach {
  enabled = hostLibDir != null
  if (hostLibDir != null) systemProperty("java.library.path", hostLibDir)
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
  implementation(libs.androidx.core.ktx)
  implementation(libs.androidx.customview)
  testImplementation(libs.junit)
  androidTestImplementation(libs.junit)
  androidTestImplementation(libs.androidx.test.runner)
  androidTestImplementation(libs.androidx.test.ext.junit)
}
