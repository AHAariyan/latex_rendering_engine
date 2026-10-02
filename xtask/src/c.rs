//! C SDK for desktop and server hosts (C, C++, and any language with a C
//! FFI: Python ctypes, Go cgo, C#, Java FFM): the header, static and shared
//! libraries, pkg-config and CMake files, and a runnable example.

use crate::util::*;

pub fn build(verify: bool) -> Result {
    let version = version();
    // A Mac build is universal (both architectures in one binary).
    let host = if cfg!(target_os = "macos") {
        "universal-apple-darwin".to_string()
    } else {
        host_target()
    };
    let name = format!("mathcore-c-{version}-{host}");
    reset_dir(&dist().join("c"))?;
    let out = dist().join("c").join(&name);
    reset_dir(&out)?;

    step(&format!("C: libraries for {host}"));
    let (static_name, shared_name) = if cfg!(target_os = "windows") {
        ("mathcore_ffi.lib", "mathcore_ffi.dll")
    } else if cfg!(target_os = "macos") {
        ("libmathcore_ffi.a", "libmathcore_ffi.dylib")
    } else {
        ("libmathcore_ffi.a", "libmathcore_ffi.so")
    };
    if cfg!(target_os = "macos") {
        // One universal binary for both Mac architectures.
        let targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
        rustup_targets(&targets)?;
        for t in targets {
            cargo_sdk_build("mathffi", Some(t))?;
        }
        std::fs::create_dir_all(out.join("lib")).map_err(|e| e.to_string())?;
        for lib in [static_name, shared_name] {
            let mut c = cmd("lipo");
            c.arg("-create");
            for t in targets {
                c.arg(sdk_out(Some(t)).join(lib));
            }
            run(c.arg("-output").arg(out.join("lib").join(lib)))?;
        }
        run(cmd("strip").arg("-S").arg(out.join("lib").join(static_name)))?;
        // Consumers find the library through @rpath.
        run(cmd("install_name_tool")
            .arg("-id")
            .arg(format!("@rpath/{shared_name}"))
            .arg(out.join("lib").join(shared_name)))?;
    } else {
        cargo_sdk_build("mathffi", None)?;
        for lib in [static_name, shared_name] {
            copy(&sdk_out(None).join(lib), &out.join("lib").join(lib))?;
        }
    }
    copy(&root().join("crates/mathffi/include/mathcore.h"), &out.join("include/mathcore.h"))?;
    copy(&root().join("LICENSE"), &out.join("LICENSE"))?;
    write(&out.join("lib/pkgconfig/mathcore.pc"), &pkgconfig(&version))?;
    write(&out.join("lib/cmake/mathcore/mathcoreConfig.cmake"), CMAKE_CONFIG)?;
    write(
        &out.join("lib/cmake/mathcore/mathcoreConfigVersion.cmake"),
        &cmake_version(&version),
    )?;
    write(&out.join("examples/render.c"), EXAMPLE)?;
    write(&out.join("README.md"), &readme(&version))?;

    if verify {
        step("C: compile and run the example against the static library");
        let exe = root().join("target/sdk-c-example");
        let mut c = cmd("cc");
        c.arg("-std=c99").arg("-Wall").arg("-Werror").arg("-I").arg(out.join("include"));
        c.arg(out.join("examples/render.c"))
            .arg(out.join("lib").join(static_name))
            .arg("-o")
            .arg(&exe);
        if cfg!(target_os = "macos") {
            c.args(["-framework", "CoreFoundation", "-framework", "Security"]);
        } else {
            c.args(["-lpthread", "-ldl", "-lm"]);
        }
        run(&mut c)?;
        let stdout = output(&mut cmd(exe.to_str().unwrap()))?;
        eprintln!("{stdout}");
        for expect in ["items", "speech: x squared", "asciimath: \\frac{a}{b}"] {
            if !stdout.contains(expect) {
                return Err(format!("C example output lacks `{expect}`"));
            }
        }
    }

    step("C: archive");
    run(cmd("tar")
        .arg("-czf")
        .arg(format!("{name}.tar.gz"))
        .arg(&name)
        .current_dir(dist().join("c")))?;
    eprintln!("\nC SDK {version} in {}", out.display());
    Ok(())
}

fn pkgconfig(version: &str) -> String {
    let libs = if cfg!(target_os = "macos") {
        "-lmathcore_ffi -framework CoreFoundation -framework Security"
    } else {
        "-lmathcore_ffi -lpthread -ldl -lm"
    };
    format!(
        "prefix=${{pcfiledir}}/../..\nlibdir=${{prefix}}/lib\nincludedir=${{prefix}}/include\n\n\
         Name: mathcore\nDescription: Native TeX math typesetting engine\nVersion: {version}\n\
         Libs: -L${{libdir}} {libs}\nCflags: -I${{includedir}}\n"
    )
}

const CMAKE_CONFIG: &str = r#"# find_package(mathcore) support. Provides the imported target mathcore::mathcore
# (static library) and mathcore::shared.
get_filename_component(_mathcore_prefix "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)
if(NOT TARGET mathcore::mathcore)
  add_library(mathcore::mathcore STATIC IMPORTED)
  if(WIN32)
    set(_mathcore_static "${_mathcore_prefix}/lib/mathcore_ffi.lib")
  else()
    set(_mathcore_static "${_mathcore_prefix}/lib/libmathcore_ffi.a")
  endif()
  set_target_properties(mathcore::mathcore PROPERTIES
    IMPORTED_LOCATION "${_mathcore_static}"
    INTERFACE_INCLUDE_DIRECTORIES "${_mathcore_prefix}/include")
  if(APPLE)
    set_property(TARGET mathcore::mathcore PROPERTY INTERFACE_LINK_LIBRARIES "-framework CoreFoundation" "-framework Security")
  elseif(UNIX)
    set_property(TARGET mathcore::mathcore PROPERTY INTERFACE_LINK_LIBRARIES pthread dl m)
  endif()
endif()
"#;

fn cmake_version(version: &str) -> String {
    format!(
        "set(PACKAGE_VERSION \"{version}\")\n\
         if(PACKAGE_FIND_VERSION VERSION_GREATER PACKAGE_VERSION)\n  set(PACKAGE_VERSION_COMPATIBLE FALSE)\nelse()\n  set(PACKAGE_VERSION_COMPATIBLE TRUE)\nendif()\n"
    )
}

const EXAMPLE: &str = r#"/* Renders a formula and prints what a host would draw.
 *   cc -I include examples/render.c lib/libmathcore_ffi.a -o render   (plus the system libraries in mathcore.pc) */
#include <stdio.h>
#include "mathcore.h"

int main(void) {
    MathEngine* engine = math_engine_new_bundled();
    if (!engine) { fprintf(stderr, "engine: %s\n", math_last_error()); return 1; }

    MathResult* r = math_engine_render(engine, "x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}",
                                       32.0f, true, 0x000000FFu, NULL, 0.0f, false);
    if (!r) { fprintf(stderr, "render: %s\n", math_last_error()); return 1; }
    printf("%zu items, %.1f x %.1f px\n", r->count, r->width, r->ascent + r->descent);
    for (size_t i = 0; i < r->count && i < 3; i++) {
        const MathItem* it = &r->items[i];
        printf("  kind %u glyph %u at (%.1f, %.1f)\n", it->kind, it->glyph, it->x, it->y);
    }
    math_result_free(r);

    char* speech = math_speech("x^2 + y^2 = z^2", NULL);
    printf("speech: %s\n", speech);
    math_string_free(speech);

    char* tex = math_asciimath_to_tex("a/b");
    printf("asciimath: %s\n", tex);
    math_string_free(tex);

    if (!math_engine_render(engine, "\\nosuchcommand", 32.0f, true, 0x000000FFu, NULL, 0.0f, false))
        printf("error reported: %s\n", math_last_error());

    math_engine_free(engine);
    return 0;
}
"#;

fn readme(version: &str) -> String {
    format!(
        r#"# mathcore C SDK {version}

Native TeX math typesetting: TeX in, a display list (glyph ids with positions,
rules, lines) out. Draw it with any canvas; glyph outlines come from
`math_engine_glyph_outline`, so the host never parses the font.

- `include/mathcore.h`: the whole API, documented.
- `lib/`: static and shared libraries.
- `lib/pkgconfig/mathcore.pc`: `pkg-config --cflags --libs mathcore`.
- `lib/cmake/mathcore`: `find_package(mathcore)` then link `mathcore::mathcore`.
- `examples/render.c`: renders a formula, prints speech and AsciiMath.

Thread safety: one `MathEngine` per thread, or serialise calls to it.
Errors: functions return NULL; `math_last_error()` says why.
"#
    )
}
