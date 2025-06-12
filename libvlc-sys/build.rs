use std::env;
use std::ffi::OsString;

use std::path::PathBuf;

#[derive(Debug, Clone)]
struct VLCAppConfig {
    include_dir: Option<PathBuf>,
    lib_dir: Option<PathBuf>,
    lib_files: Option<Vec<PathBuf>>,
    plugins_dir: Option<PathBuf>,
}

#[cfg(feature = "pkg_config")]
fn pkg_config_probe() -> Result<pkg_config::Library, pkg_config::Error> {
    pkg_config::Config::new()
        .print_system_libs(false)
        .probe("libvlc")
}

#[cfg(feature = "bindgen")]
fn generate_bindings() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let mut bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Use libc
        .ctypes_prefix("libc")
        // Allowlist
        .allowlist_type(".*vlc.*")
        .allowlist_function(".*vlc.*")
        .allowlist_var(".*VLC.*")
        .allowlist_var(".*vlc.*")
        .allowlist_var("^LIBVLC_.*") // Regex for all vars starting with LIBVLC_
        .allowlist_function("vsnprintf")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    // add includes
    for include_path in header_includes() {
        bindings = bindings.clang_arg(format!("-I{}", include_path.display()));
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

fn header_includes() -> Vec<PathBuf> {
    let mut includes = Vec::new();

    // Check for VLC_INCLUDE_DIR environment variable

    if let Some(include_dir) = env::var_os("VLC_INCLUDE_DIR") {
        includes.push(PathBuf::from(include_dir));
    }

    #[cfg(feature = "pkg_config")]
    {
        if let Ok(lib) = pkg_config_probe() {
            for include_path in lib.include_paths {
                includes.push(include_path);
            }
        }
    }

    // If VLC_INCLUDE_DIR is not set, use the default include path based on the OS
    let config = vlc_config();
    if let Some(default_path) = config.include_dir {
        includes.push(default_path);
    }

    includes
}

fn link_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Check for VLC_LIB_DIR environment variable
    if let Some(lib_dir) = vlc_env_lib_path() {
        dirs.push(lib_dir);
    }

    #[cfg(feature = "pkg_config")]
    {
        if let Ok(lib) = pkg_config_probe() {
            for include_path in lib.link_paths {
                dirs.push(include_path);
            }
        }
    }

    // If VLC_LIB_DIR is not set, use the default path based on the OS
    let config = vlc_config();
    if let Some(default_path) = config.lib_dir {
        dirs.push(default_path);
    }

    dirs
}

#[cfg(target_os = "windows")]
fn vlc_config() -> VLCAppConfig {
    // On Windows, we assume the default path is in the VLC installation directory
    let vlc_path = env::var_os("VLC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let program_files =
                env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
            let vlc_path = PathBuf::from(program_files).join("VideoLAN").join("VLC");
            vlc_path
        });

    // SDK is where the include and lib directories are located on windows
    let sdk = vlc_path.join("sdk");

    let include_dir = sdk.join("include");
    let lib_dir = sdk.join("lib");
    let plugins_dir = vlc_path.join("plugins");

    let libs = vec![lib_dir.join("libvlc.lib"), lib_dir.join("libvlccore.lib")];

    VLCAppConfig {
        include_dir: include_dir.exists().then_some(include_dir),
        lib_dir: lib_dir.exists().then_some(lib_dir),
        lib_files: Some(libs), // Windows does not use lib files in the same way as Unix-like systems
        plugins_dir: plugins_dir.exists().then_some(plugins_dir),
    }
}

#[cfg(target_os = "macos")]
fn vlc_config() -> VLCAppConfig {
    // On macOS, we assume the default path is in the VLC installation directory
    let vlc_path = env::var_os("VLC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Applications/VLC.app/Contents/"));

    let mut include_dir = vlc_path.join("MacOS").join("include");
    let mut lib_dir = vlc_path.join("MacOS").join("lib");
    let plugins_dir = vlc_path.join("Frameworks").join("plugins");

    if !include_dir.exists() {
        include_dir = vlc_path.join("include");
    }

    if !lib_dir.exists() {
        lib_dir = vlc_path.join("lib");
    }

    let mut libs = vec![
        lib_dir.join("libvlc.dylib"),
        lib_dir.join("libvlccore.dylib"),

        // lib_dir.join("libvlc"),
        // lib_dir.join("libvlccore"),
        // lib_dir.join("vlc"),
        // lib_dir.join("vlccore"),
    ];

    #[cfg(feature = "pkg_config")]
    {
        let pkg = pkg_config_probe();
        if let Ok(lib) = pkg {
            if !include_dir.exists() {
                // If the include/lib directory does not exist, we can use pkg-config to find it
                include_dir = lib.include_paths.first().cloned().unwrap_or(include_dir);
            }

            if !lib_dir.exists() {
                // If the lib directory does not exist, we can use pkg-config to find it
                lib_dir = lib.link_paths.first().cloned().unwrap_or(lib_dir);
            }

            libs.extend(lib.link_paths);
        }
    }

    // let default_lib_dir = PathBuf::from("/Applications/VLC.app/Contents/MacOS/lib");
    // let default_include_dir = PathBuf::from("/Applications/VLC.app/Contents/MacOS/include");

    VLCAppConfig {
        include_dir: include_dir.exists().then_some(include_dir),
        lib_dir: lib_dir.exists().then_some(lib_dir),
        lib_files: Some(libs),
        plugins_dir: plugins_dir.exists().then_some(plugins_dir),
    }
}

#[cfg(target_os = "linux")]
fn vlc_config() -> VLCAppConfig {
    // On Linux, we assume the default path is in the system library paths
    let vlc_path = env::var_os("VLC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr"));

    // This is usually handled by pkg-config, so we don't need to specify it here.

    let mut libs = vec![PathBuf::from("libvlc.so"), PathBuf::from("libvlccore.so")];

    #[cfg(feature = "pkg_config")]
    {
        let pkg = pkg_config_probe();
        if let Ok(lib) = pkg {
            libs.extend(lib.link_paths);
        }
    }

    VLCAppConfig {
        include_dir: None,
        lib_dir: None,
        lib_files: Some(libs),
        plugins_dir: None,
    }
}

fn vlc_env_lib_path() -> Option<PathBuf> {
    let arch_path: Option<OsString> = match target_arch().as_str() {
        "x86" => env::var_os("VLC_LIB_DIR_X86"),
        "x86_64" => env::var_os("VLC_LIB_DIR_X86_64"),
        "arm" => env::var_os("VLC_LIB_DIR_ARM"),
        "aarch64" => env::var_os("VLC_LIB_DIR_AARCH64"),
        _ => unreachable!(),
    };

    arch_path
        .or_else(|| env::var_os("VLC_LIB_DIR"))
        .map(PathBuf::from)
}

fn target_arch() -> String {
    env::var("CARGO_CFG_TARGET_ARCH").unwrap()
}

#[cfg(feature = "runtime")]
fn copy_runtime() {
    let config = vlc_config();

    // copy the runtime libraries to the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    if let (Some(lib_dir), Some(lib_files)) = (&config.lib_dir, &config.lib_files) {
        for lib in lib_files {
            let src = lib_dir.join(lib);
            let dest = out_dir.join(lib.file_name().unwrap());
            if !src.exists() {
                eprintln!("Warning: Runtime library {} does not exist, skipping copy.", src.display());
                continue;
            }
            std::fs::copy(src, dest).expect("Failed to copy runtime library");
        }
    }

    // copy plugins
    if let Some(plugins_dir) = &config.plugins_dir {
        let out_plugins_dir = out_dir.join("vlc_plugins");
        std::fs::create_dir_all(&out_plugins_dir).expect("Failed to create plugins directory");

        fs_extra::dir::copy(
            plugins_dir,
            &out_plugins_dir,
            &fs_extra::dir::CopyOptions {
                overwrite: true,
                ..Default::default()
            },
        )
        .expect("Failed to copy VLC plugins");
    }
}

fn main() {
    let config = vlc_config();

    println!("VLC configuration: {:#?}", config);

    // Binding generation
    #[cfg(feature = "bindgen")]
    generate_bindings();

    #[cfg(not(feature = "bindgen"))]
    copy_pregenerated_bindings();

    for lib_dir in link_directories() {
        println!(
            "cargo:rustc-link-search=native={}",
            lib_dir.to_string_lossy()
        );
    }

    // for lib in config.lib_files.unwrap_or_default() {
    //     println!(
    //         "cargo:rustc-link-lib=dylib={}",
    //         lib.file_stem().unwrap().to_string_lossy()
    //     );
    // }

    {
        // On macOS, we need to link against the dynamic libraries
        println!("cargo:rustc-link-lib=dylib=vlc");
        println!("cargo:rustc-link-lib=dylib=vlccore");
    }

    #[cfg(feature = "runtime")]
    copy_runtime();
}
