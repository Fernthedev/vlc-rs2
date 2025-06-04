use pkg_config;
use std::env;
use std::path::PathBuf;

#[cfg(feature = "bindgen")]
fn generate_bindings() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // For no_std
        .use_core()
        // Use libc
        .ctypes_prefix("libc")
        // Allowlist
        .allowlist_type(".*vlc.*")
        .allowlist_function(".*vlc.*")
        .allowlist_var(".*vlc.*")
        .allowlist_function("vsnprintf")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // Set header include paths
    let pkg_config_library = pkg_config::Config::new().probe("libvlc");
    match pkg_config_library {
        // try with pkg-config
        Ok(pkg_config_library) => {
            for include_path in &pkg_config_library.include_paths {
                bindings = bindings.clang_arg(format!("-I{}", include_path.display()));
            }
        }
        // fallback to manually set include path
        _ => {
            // If pkg-config fails, we assume the user has set VLC_INCLUDE_DIR
            #[cfg(target_os = "macos")]
            {
                bindings = bindings.clang_arg(format!("-I{}", macos::include_dir()));
            }
        }
    }

    let bindings = bindings.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[cfg(not(feature = "bindgen"))]
fn copy_pregenerated_bindings() {
    use std::fs;

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let crate_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    fs::copy(crate_path.join("bindings.rs"), out_path.join("bindings.rs"))
        .expect("Couldn't find pregenerated bindings!");
}

fn link_vlc_with_pkgconfig() -> Result<pkg_config::Library, pkg_config::Error> {
    pkg_config::Config::new()
        .print_system_libs(false)
        .probe("libvlc")
}

#[cfg(target_os = "windows")]
mod windows {
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    compile_error!("Only x86 and x86_64 are supported at the moment. Adding support for other architectures should be trivial.");

    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use vswhom::VsFindResult;

    pub fn link_vlc() {
        let vlc_path = vlc_path();

        let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

        let vs = VsFindResult::search().expect("Could not locate Visual Studio");
        let vs_exe_path = PathBuf::from(
            vs.vs_exe_path
                .expect("Could not retrieve executable path for Visual Studio"),
        );

        generate_lib_from_dll(&out_dir, &vs_exe_path, &vlc_path);
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        // NOTE: Without this directive, linking fails with:
        //       ```
        //       error LNK2019: unresolved external symbol vsnprintf referenced in function _{MangledSymbolName}
        //          msvcrt.lib(vsnprintf.obj) : error LNK2001: unresolved external symbol vsnprintf
        //          msvcrt.lib(vsnprintf.obj) : error LNK2001: unresolved external symbol _vsnprintf
        //       ```
        //       https://stackoverflow.com/a/34230122
        println!("cargo:rustc-link-lib=dylib=legacy_stdio_definitions");
    }

    fn generate_lib_from_dll(out_dir: &Path, vs_exe_path: &Path, vlc_path: &Path) {
        // https://wiki.videolan.org/GenerateLibFromDll/

        let vs_dumpbin = vs_exe_path.join("dumpbin.exe");
        let vs_lib = vs_exe_path.join("lib.exe");
        let vlc_def_path = out_dir.join("libvlc.def");
        let vlc_import_lib = out_dir.join("vlc.lib");

        let libvlc = vlc_path.join("libvlc.dll");
        let exports = Command::new(vs_dumpbin)
            .current_dir(out_dir)
            .arg("/EXPORTS")
            .arg(libvlc.display().to_string().trim_end_matches(r"\"))
            .output()
            .unwrap();
        let exports = String::from_utf8(exports.stdout).unwrap();

        let mut vlc_def = String::from("EXPORTS\n");
        for line in exports.lines() {
            if let Some(line) = line.get(26..) {
                if line.starts_with("libvlc_") {
                    vlc_def.push_str(line);
                    vlc_def.push_str("\r\n");
                }
            }
        }
        fs::write(&vlc_def_path, vlc_def.into_bytes()).unwrap();

        // FIXME: Handle paths with spaces in them.
        Command::new(vs_lib)
            .current_dir(out_dir)
            .arg("/NOLOGO")
            .args(&[
                format!(
                    r#"/DEF:{}"#,
                    vlc_def_path.display().to_string().trim_end_matches(r"\")
                ),
                format!(
                    r#"/OUT:{}"#,
                    vlc_import_lib.display().to_string().trim_end_matches(r"\")
                ),
                format!(
                    "/MACHINE:{}",
                    match target_arch().as_str() {
                        "x86" => "x86",
                        "x86_64" => "x64",
                        _ => unreachable!(),
                    }
                ),
            ])
            .spawn()
            .unwrap();
    }

    fn vlc_path() -> PathBuf {
        #[allow(unused_assignments)]
        let arch_path: Option<OsString> = match target_arch().as_str() {
            "x86" => env::var_os("VLC_LIB_DIR_X86"),
            "x86_64" => env::var_os("VLC_LIB_DIR_X86_64"),
            _ => unreachable!(),
        };

        arch_path
            .or_else(|| env::var_os("VLC_LIB_DIR"))
            .map(PathBuf::from)
            .expect("VLC_LIB_DIR not set")
    }

    fn target_arch() -> String {
        env::var("CARGO_CFG_TARGET_ARCH").unwrap()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::env;

    pub fn link_vlc() {
        let vlc_path = env::var_os("VLC_LIB_DIR")
            .unwrap_or_else(|| "/Applications/VLC.app/Contents/MacOS/lib".into());

        println!("cargo:rustc-link-search=native={:?}", vlc_path);
        println!("cargo:rustc-link-lib=dylib=vlc");
        println!("cargo:rustc-link-lib=dylib=vlccore");
    }

    pub fn include_dir() -> String {
        env::var("VLC_INCLUDE_DIR")
            .unwrap_or_else(|_| "/Applications/VLC.app/Contents/MacOS/include".into())
    }
}

fn main() {
    // Binding generation
    #[cfg(feature = "bindgen")]
    generate_bindings();

    #[cfg(not(feature = "bindgen"))]
    copy_pregenerated_bindings();

    // Link
    if let Err(err) = link_vlc_with_pkgconfig() {
        #[cfg(target_os = "windows")]
        windows::link_vlc();

        #[cfg(target_os = "macos")]
        macos::link_vlc();
    }

    println!("cargo:rustc-link-lib=vlc");
}
