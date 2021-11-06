use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use log::info;
use serde::{Deserialize, Serialize};

use file_meta_data::FileMetaData;
use godot_version::GodotVersion;

mod file_meta_data;
mod godot_version;
mod test_asset_pack;

const ASSET_PACK_MAGIC_FILE_HEADER: [u8; 4] = [0x47, 0x44, 0x50, 0x43];
const I32: usize = 4;
const I64: usize = 8;
const GODOT_METADATA_RESERVED_SPACE: usize = 16 * I32;
const MD5_BYTES: usize = 16;

const RESOURCE_PATH_PREFIX: &str = "res://";
const ASSET_PACK_PREFIX: &str = "packs/";
const PACK_FILE_NAME: &str = "pack.json";
const TAGS_FILE_NAME: &str = "data/default.dungeondraft_tags";
const OBJECT_FILES_PREFIX: &str = "textures/objects/";

const NAME_KEY: &str = "name";
const ID_KEY: &str = "name";
const VERSION_KEY: &str = "version";
const AUTHOR_KEY: &str = "author";

#[derive(Debug)]
pub struct AssetPack {
    pub godot_version: GodotVersion,
    pub meta: PackMeta,
    pub tags: Tags,
    pub object_files: HashMap<String, Vec<u8>>,
    pub other_files: HashMap<String, Vec<u8>>,
}

impl AssetPack {
    pub fn from_read<R: Read + Seek>(data: &mut R) -> Result<Self> {
        data.seek(SeekFrom::Start(ASSET_PACK_MAGIC_FILE_HEADER.len() as u64))?;

        let godot_version =
            GodotVersion::from_read(data).context("Could not read godot version")?;
        data.read_exact(&mut [0; GODOT_METADATA_RESERVED_SPACE])?;

        let nr_of_files = data.read_i32::<LE>()? as usize;

        let mut files_meta = vec![];

        for i in 0..nr_of_files {
            let file_meta = FileMetaData::from_read(data).context(format!(
                "Could not read file metadata of file {} from {}",
                i + 1,
                nr_of_files
            ))?;

            files_meta.push(file_meta);
        }

        files_meta.sort();

        let mut object_files = HashMap::new();
        let mut other_files = HashMap::new();
        let mut maybe_meta = None;
        let mut maybe_tags = None;

        for meta in files_meta {
            let mut file_data = vec![0; meta.size];
            data.read_exact(&mut file_data)?;

            let pathbuf = &PathBuf::from(meta.path.clone());

            // A dungeondraft asset pack for some reason has two json files with identical contents
            // one is the root json file `packs/<pack-id>.json` and the other
            // is `packs/<pack-id>/pack.json`. This is why whe ignore the second one
            // (via `is_pack_file()`)
            if is_root_json_file(pathbuf) {
                maybe_meta = Some(serde_json::from_slice(&file_data)?);
            } else if is_tags_file(&meta.path) {
                maybe_tags = Some(serde_json::from_slice(&file_data)?);
            } else if is_objects_file(&meta.path) {
                object_files.insert(meta.path.clone(), file_data);
            } else if !is_pack_file(pathbuf) {
                other_files.insert(meta.path.clone(), file_data);
            }
        }

        Ok(AssetPack {
            godot_version,
            meta: maybe_meta.unwrap(),
            tags: maybe_tags.unwrap(),
            object_files,
            other_files,
        })
    }

    pub fn to_write<W: Write>(&self, data: &mut W) -> Result<()> {
        data.write_all(&ASSET_PACK_MAGIC_FILE_HEADER)?;
        self.godot_version.to_write(data)?;
        data.write_all(&[0; GODOT_METADATA_RESERVED_SPACE])?;

        let file_path_prefix =
            RESOURCE_PATH_PREFIX.to_owned() + ASSET_PACK_PREFIX + self.meta.id.as_str();

        let pack_meta_file = serde_json::to_vec(&self.meta)?;
        let root_pack_file_metadata =
            FileMetaData::new(file_path_prefix.clone() + ".json", pack_meta_file.len());
        let pack_file_metadata = FileMetaData::new(
            file_path_prefix.clone() + "/" + PACK_FILE_NAME,
            pack_meta_file.len(),
        );

        let tags_file = serde_json::to_vec(&self.tags)?;
        let tags_metadata = FileMetaData::new(
            file_path_prefix.clone() + "/" + TAGS_FILE_NAME,
            tags_file.len(),
        );

        let mut files = vec![];
        // A dungeondraft asset pack for some reason has two json files with identical contents
        // one is the root json file `packs/<pack-id>.json` and the other
        // is `packs/<pack-id>/pack.json`.
        // This is why we add two files with the same content here.
        files.push((root_pack_file_metadata, &pack_meta_file));
        files.push((pack_file_metadata, &pack_meta_file));
        files.push((tags_metadata, &tags_file));

        for (file_path, data) in self.object_files.iter().chain(self.other_files.iter()) {
            let path_with_prefix = file_path_prefix.clone() + "/" + file_path;

            files.push((FileMetaData::new(path_with_prefix, data.len()), data));
        }

        data.write_i32::<LE>(files.len() as i32)?;

        let mut file_offset = Self::calculate_files_block_starting_offset(&files);

        for (meta, _) in files.iter_mut() {
            meta.offset = file_offset as u64;
            file_offset += meta.size;
        }

        for (meta, _) in files.iter() {
            meta.to_write(data)?;
        }

        for (_, file_data) in files.iter() {
            data.write_all(file_data)?;
        }

        Ok(())
    }

    fn calculate_files_block_starting_offset(files: &Vec<(FileMetaData, &Vec<u8>)>) -> usize {
        // The i32 is where the amount of files is kept.
        let mut file_offset = ASSET_PACK_MAGIC_FILE_HEADER.len()
            + GodotVersion::size_in_bytes()
            + GODOT_METADATA_RESERVED_SPACE
            + I32;

        for (meta, _) in files.iter() {
            file_offset += meta.calculate_binary_size();
        }

        file_offset
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PackMeta {
    pub name: String,
    pub id: String,
    pub version: String,
    pub author: String,
    pub custom_color_overrides: ColorOverrides,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct ColorOverrides {
    pub enabled: bool,
    pub min_redness: f32,
    pub min_saturation: f32,
    pub red_tolerance: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Tags {
    tags: HashMap<String, Vec<String>>,
    sets: HashMap<String, Vec<String>>,
}

fn read_string(data: &mut dyn Read, length: usize) -> Result<String> {
    let mut bytes = vec![0; length];
    data.read_exact(bytes.as_mut_slice())
        .context("Could not read string")?;

    Ok(String::from_utf8(bytes).context("Could not convert string from bytes")?)
}

fn is_file_asset_pack(pack: &PathBuf) -> Result<bool> {
    let mut file = std::fs::File::open(pack)?;

    let mut magic_file_number = [0; 4];
    file.read_exact(&mut magic_file_number)?;

    Ok(magic_file_number == ASSET_PACK_MAGIC_FILE_HEADER)
}

pub fn read_asset_pack(pack_path: &PathBuf) -> Result<AssetPack> {
    info!("Reading '{}'", pack_path.display());

    let mut pack_file = std::fs::File::open(pack_path).context("Asset pack file does not exist")?;
    let pack = AssetPack::from_read(&mut pack_file).context("Could not read metadata")?;

    info!("Godot package version: {}", pack.godot_version);
    info!("Files in package: {}", pack.other_files.len());

    info!("Pack name: {}", pack.meta.name);
    info!("Pack author: {}", pack.meta.author);
    info!("Pack version: {}", pack.meta.version);
    info!("Pack id: {}", pack.meta.id);

    info!("Tags: {:?}", pack.tags);

    Ok(pack)
}

/// Returns true for `<pack-id>.json` files without any parent directory.
fn is_root_json_file(path: &PathBuf) -> bool {
    path.extension().unwrap_or(OsStr::new("")) == OsStr::new("json")
        && path.parent() == Some(Path::new(""))
}

/// Returns true for `pack.json` files, regardless of parent directory.
fn is_pack_file(path: &PathBuf) -> bool {
    path.file_name() == Some(OsStr::new(PACK_FILE_NAME))
}

/// Returns true for `data/default.dungeondraft_tags` files, regardless of parent dir.
fn is_tags_file(path: &str) -> bool {
    path.ends_with(TAGS_FILE_NAME)
}

/// Returns true if path starts with `textures/objects/`.
fn is_objects_file(path: &str) -> bool {
    path.starts_with(OBJECT_FILES_PREFIX)
}

fn unpack_file(read: &mut dyn Read, file_size: usize) -> Result<Vec<u8>> {
    let mut file_data = vec![0; file_size];
    read.read_exact(file_data.as_mut_slice())?;

    Ok(file_data)
}

#[cfg(test)]
mod test {
    use std::io::{Cursor, Write};
    use std::path::PathBuf;

    use byteorder::{WriteBytesExt, LE};

    use crate::asset_pack::file_meta_data::FileMetaData;
    use crate::asset_pack::{is_root_json_file, MD5_BYTES};

    #[test]
    fn packed_file_from_read() {
        let path = String::from("res://X3DLFK/test/bla.txt");
        let offset = 12;
        let size = 987;
        let md5 = [12; MD5_BYTES];

        let mut data = vec![];
        data.write_i32::<LE>(path.len() as i32).unwrap();
        data.write_all(path.as_bytes()).unwrap();
        data.write_i64::<LE>(offset).unwrap();
        data.write_i64::<LE>(size).unwrap();
        data.write_all(&md5).unwrap();

        let mut cursor = Cursor::new(data);
        let file = FileMetaData::from_read(&mut cursor).unwrap();

        assert_eq!(file.path, "test/bla.txt");
        assert_eq!(file.offset, offset as u64);
        assert_eq!(file.size, size as usize);
        assert_eq!(file.md5, md5);
    }

    #[test]
    fn test_is_root_json_file() {
        assert!(is_root_json_file(&PathBuf::from("8UWKyQPf.json")));
        assert!(!is_root_json_file(&PathBuf::from("bla/8UWKyQPf.json")));
        assert!(!is_root_json_file(&PathBuf::from("8UWKyQPf.txt")));
    }
}
