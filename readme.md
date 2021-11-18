This (over-engineered) command line program can currently read a complete folder of asset packs, 
clean any object tags / tag sets that don't have any objects associated with them,
and then output the cleaned packs into a different folder.

In the future the plan is to also be able to re-tag assets en-masse.

Works on Linux. Should also work on Windows, but hasn't been tested there.

- Basic usage: `dd_asset_tools <INPUT_DIR> <OUTPUT_DIR>`
- Add `-F` to overwrite existing packs in the output directory.
- `dd_asset_tools -h` shows additional help info.

[Download the executables from here](https://github.com/Wcubed/dungeondraft-asset-tools/releases)

__Do not redistribute asset packs without the permission of the original creator__.

# Changelog

## 0.1.0
- Can clean empty tags and tag groups from a folder of asset files.

# Known issues

- Cannot read asset packs that use unescaped backslashes as path separators in their `.json` files.
This is not an issue with the tool, but with the asset pack.
Using backslashes as path separators is not actually valid json.
Will output a warning when encountering such a pack, and skip it.

# Building

For most people it is not necessary to build the source-code yourself.
Instead, you can download the latest release from [here](https://github.com/Wcubed/dungeondraft-asset-tools/releases).

## Building for your own system
To build the program on your own system:
- Install [rust](https://www.rust-lang.org/)
- Run `cargo build --release` in the project's base directory.
- The executable should now be located in `target/release/`

## Cross-compiling for all supported systems at once
To cross-compile from linux to windows (and any future supported OS's), first install the following tools:
- [cross](https://github.com/rust-embedded/cross). Rust cross-compilation tool.
  - `cargo install cross`
  - Add the `cross` executable to your `PATH`. The executable should be located here: `/home/<username>/.cargo/bin/cross`
  - Install `podman` version 1.6.3 or later (`docker` also works)
- [cargo-make](https://github.com/sagiegurari/cargo-make#usage-predefined-makefiles). Rust task runner and build tool.
  - `cargo install cargo-make`

Now whenever you want to build all the versions:
- `cargo make build-release-all`
It will first run the unit tests for all systems, and then create the executables.
- The executables are located in `target/<compilation-target>/release/`.
For example, the windows executable will be located at: `target/x86_64-pc-windows-gnu/release/dd_asset_tools.exe`

# Reading dungeondraft pack files

The `*.dungeondraft_pack` files are actually godot package files.

The format is as follows:

- 0x43504447: Magic number (GDPC)
- 4 x Int32: Engine version: version, major, minor, revision
- 16 x Int32: Reserved space, 0
- Int32: Number of files in archive
- For each file:
  - Int32: Length of path string
  - String: Path as string, e.g. res://packs/XKLS89KL/pack.json
  - Int64: File offset
  - Int64: File size
  - 16 bytes: MD5
- All file contents.