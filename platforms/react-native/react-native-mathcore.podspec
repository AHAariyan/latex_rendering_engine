require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name         = "react-native-mathcore"
  s.version      = package["version"]
  s.summary      = package["description"]
  s.homepage     = "https://github.com/AHAariyan/latex_rendering_engine"
  s.license      = package["license"]
  s.authors      = { "mathcore" => "mathcore@users.noreply.github.com" }
  s.platforms    = { :ios => "15.1" }
  s.source       = { :git => "https://github.com/AHAariyan/latex_rendering_engine.git", :tag => "v#{s.version}" }

  # ios/MathCore holds the engine's Swift sources and ios/MathCoreFFI.xcframework
  # the engine itself, both put there by `cargo xtask sdk react-native`.
  s.source_files = "ios/**/*.{h,m,mm,swift}"
  # The Objective-C++ headers import C++ codegen headers. Kept private, they
  # stay out of the module Swift interop builds, which is Objective-C only.
  s.private_header_files = "ios/**/*.h"
  s.vendored_frameworks = "ios/MathCoreFFI.xcframework"
  s.swift_version = "5.9"
  s.pod_target_xcconfig = { "DEFINES_MODULE" => "YES" }

  install_modules_dependencies(s)
end
