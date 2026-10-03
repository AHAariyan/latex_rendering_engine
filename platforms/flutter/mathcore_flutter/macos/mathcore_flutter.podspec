# FFI plugin: the engine as a dynamic framework (MathCoreFFI.xcframework),
# opened by name from Dart. Built by `cargo xtask sdk flutter`.
Pod::Spec.new do |s|
  s.name             = 'mathcore_flutter'
  s.version          = '0.1.0'
  s.summary          = 'Native TeX math rendering for Flutter.'
  s.homepage         = 'https://github.com/AHAariyan/latex_rendering_engine'
  s.license          = { :type => 'MIT' }
  s.author           = { 'mathcore' => 'mathcore@users.noreply.github.com' }
  s.source           = { :path => '.' }
  s.platform         = :osx, '10.14'
  s.vendored_frameworks = 'mathcore_flutter/MathCoreFFI.xcframework'
  s.dependency 'FlutterMacOS'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
