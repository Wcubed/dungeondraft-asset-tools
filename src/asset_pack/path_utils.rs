use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub const RESOURCE_PATH_PREFIX: &str = "res://";
pub const ASSET_PACK_PREFIX: &str = "packs/";
pub const PACK_FILE_NAME: &str = "pack.json";
pub const TAGS_FILE_NAME: &str = "data/default.dungeondraft_tags";
pub const OBJECT_FILES_PREFIX: &str = "textures/objects/";

/// Returns true for `<pack-id>.json` files without any parent directory.
pub fn is_root_json_file(path: &PathBuf) -> bool {
    path.extension().unwrap_or(OsStr::new("")) == OsStr::new("json")
        && path.parent() == Some(Path::new(""))
}

/// Returns true for `pack.json` files, regardless of parent directory.
pub fn is_pack_file(path: &PathBuf) -> bool {
    path.file_name() == Some(OsStr::new(PACK_FILE_NAME))
}

/// Returns true for `data/default.dungeondraft_tags` files, regardless of parent dir.
pub fn is_tags_file(path: &str) -> bool {
    path.ends_with(TAGS_FILE_NAME)
}

/// Returns true if path starts with `textures/objects/`.
pub fn is_objects_file(path: &str) -> bool {
    path.starts_with(OBJECT_FILES_PREFIX)
}
