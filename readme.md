This command line program can currently read a complete folder of asset packs, clean any object tags / tag sets that
don't have any objects associated with them, and then output the cleaned packs into a different folder.

For further usage instructions, call with the `-h` or `--help` arguments.

# Building

To build the program on your own computer:
- Install [rust](https://www.rust-lang.org/)
- Run `cargo build --release` in the project's base directory.
- The executable should now be located in `target/release/`

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