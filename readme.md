The `*.dungeondraft_pack` files are actually godot packages.

The format is as follows:

- 0x43504447: Magic number (GDPC)
- 4 x Int32: Engine version: version, major, minor, revision
- 16 x Int32: Reserved space, 0
- Int32: Number of files in archive
- For each file:
  - Int32: Length of path string
  - String: Path as string, e.g. res://actors/Enemy/enemy.atex
  - Int64: File offset
  - Int64: File size
  - 16 bytes: MD5
- All file contents.