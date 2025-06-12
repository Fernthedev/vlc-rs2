# vlc-rs [![Build Status](https://travis-ci.org/garkimasera/vlc-rs.svg?branch=master)](https://travis-ci.org/garkimasera/vlc-rs)

Rust bindings for libVLC media framework.

## Status

Many missing functions and wrappers.

## Use

Please add the following dependencies to your Cargo.toml.

```Toml
[dependencies]
vlc-rs = "0.3" # VLC 3.0
libvlc-sys = "0.3" # VLC 3.0 or 4.0
```

Or:

```Toml
[dependencies.vlc-rs]
git = "https://github.com/garkimasera/vlc-rs.git"
```

## Example

Play for 10 seconds from a media file.

```Rust
extern crate vlc;
use vlc::{Instance, Media, MediaPlayer};
use std::thread;

fn main() {
    // Create an instance
    let instance = Instance::new().unwrap();
    // Create a media from a file
    let md = Media::new_path(&instance, "path_to_a_media_file.ogg").unwrap();
    // Create a media player
    let mdp = MediaPlayer::new(&instance).unwrap();
    mdp.set_media(&md);

    // Start playing
    mdp.play().unwrap();

    // Wait for 10 seconds
    thread::sleep(::std::time::Duration::from_secs(10));
}
```

Other examples are in the examples directory.

## Building

### Windows

To build `vlc-rs`, you need to have VLC installed or available as a package.

1. Download VLC:
    - For stable releases: [videolan.org](https://www.videolan.org/vlc/download-windows.html)
    - For development builds: [nightlies.videolan.org](https://nightlies.videolan.org/build/win64/)

2. For `x86_64`, download the "Installer for 64bit version" or the 64-bit ZIP package.
    For `x86`, download the "7zip package" or "Zip package".

3. Extract the package to a directory with no spaces in the path.

4. Set the `VLC_PATH` environment variable to point to the directory containing the VLC files.

### macOS

1. Download VLC:
    - For stable releases: [videolan.org](https://www.videolan.org/vlc/download-macosx.html)
    - For development builds: [nightlies.videolan.org](https://nightlies.videolan.org/build/macosx/)

2. Install or extract VLC to your Applications folder.

3. Set the `VLC_PATH` environment variable to the VLC application directory (typically `/Applications/VLC.app`).

### Linux

1. Install VLC using your distribution's package manager:
    ```
    # Debian/Ubuntu
    sudo apt install libvlc-dev vlc

    # Fedora
    sudo dnf install vlc-devel vlc

    # Arch Linux
    sudo pacman -S vlc
    ```

2. For development builds: [nightlies.videolan.org](https://nightlies.videolan.org/build/debian/)

3. The build system should automatically use pkg-config find the VLC libraries. If not, set the `VLC_PATH` environment variable to the directory containing the VLC libraries.

### Distribution

For distributing your application, include the necessary VLC DLLs and the `plugins` directory with your executable. 
With the `runtime` feature, the necessary files are copied to `libvlc-sys`'s `OUT` path.

## License

MIT (Examples are licensed under CC0)
