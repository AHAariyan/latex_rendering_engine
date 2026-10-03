import 'dart:io';

/// The native library the tests load: `MATHCORE_LIB`, else the host build in
/// the repository's target directory.
String libraryForTests() {
  final env = Platform.environment['MATHCORE_LIB'];
  if (env != null) return env;
  final name = Platform.isMacOS
      ? 'libmathcore_ffi.dylib'
      : Platform.isWindows
          ? 'mathcore_ffi.dll'
          : 'libmathcore_ffi.so';
  for (final profile in ['sdk', 'release', 'debug']) {
    final p = '../../../target/$profile/$name';
    if (File(p).existsSync()) return p;
  }
  return '../../../target/release/$name';
}
