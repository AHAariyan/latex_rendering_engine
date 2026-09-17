# FFI plugin: links the static mathcore library built by scripts/build-ios.sh.
Pod::Spec.new do |s|
  s.name             = 'mathcore_flutter'
  s.version          = '0.1.0'
  s.summary          = 'Native TeX math rendering for Flutter.'
  s.homepage         = 'https://github.com/AHAariyan/latex_rendering_engine'
  s.license          = { :type => 'MIT' }
  s.author           = { 'mathcore' => 'noreply@example.com' }
  s.source           = { :path => '.' }
  s.platform         = :ios, '13.0'
  s.vendored_frameworks = 'MathCoreFFI.xcframework'
  s.dependency 'Flutter'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
  # Keep every C symbol so dart:ffi can find them via DynamicLibrary.process().
  s.user_target_xcconfig = { 'OTHER_LDFLAGS' => '-force_load $(PODS_XCFRAMEWORKS_BUILD_DIR)/mathcore_flutter/MathCoreFFI.framework/MathCoreFFI' }
end
