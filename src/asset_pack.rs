use anyhow::{Context, Result};
use byteorder::{ReadBytesExt, LE};
use log::info;
use serde::Deserialize;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::fmt::Formatter;
use std::fs::File;
use std::io::{Read, Write};
use std::io::{Seek, SeekFrom};
use std::path::{Path, PathBuf};

const ASSET_PACK_MAGIC_FILE_HEADER: [u8; 4] = [0x47, 0x44, 0x50, 0x43];
const I32: usize = 4;
const GODOT_METADATA_RESERVED_SPACE: usize = 16 * I32;
const MD5_BYTES: usize = 16;

const RESOURCE_PATH_PREFIX: &str = "res://";
const ASSET_PACK_PREFIX: &str = "packs/";

const NAME_KEY: &str = "name";
const ID_KEY: &str = "name";
const VERSION_KEY: &str = "version";
const AUTHOR_KEY: &str = "author";

#[derive(Debug)]
pub struct AssetPack {
    godot_version: GodotVersion,
    meta: PackMeta,
    files_meta: HashMap<String, FileMetaData>,
}

impl AssetPack {
    fn from_read<R: Read + Seek>(data: &mut R) -> Result<Self> {
        let version = GodotVersion::from_read(data).context("Could not read godot version")?;
        data.read_exact(&mut [0; GODOT_METADATA_RESERVED_SPACE])?;

        let nr_of_files = data.read_i32::<LE>()? as usize;

        let mut files_meta = HashMap::with_capacity(nr_of_files);

        let mut maybe_pack_meta_file = None;

        for i in 0..nr_of_files {
            let file_meta = FileMetaData::from_read(data).context(format!(
                "Could not read file metadata of file {} from {}",
                i + 1,
                nr_of_files
            ))?;

            if is_root_json_file(&PathBuf::from(&file_meta.path)) {
                maybe_pack_meta_file = Some(file_meta);
            } else {
                files_meta.insert(file_meta.path.clone(), file_meta);
            }
        }

        let pack_meta_file = maybe_pack_meta_file.unwrap();

        let pack_meta = PackMeta::from_read(data, &pack_meta_file)?;

        Ok(Self {
            godot_version: version,
            meta: pack_meta,
            files_meta,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
struct GodotVersion {
    version: i32,
    major: i32,
    minor: i32,
    revision: i32,
}

impl GodotVersion {
    fn new(version: i32, major: i32, minor: i32, revision: i32) -> Self {
        Self {
            version,
            major,
            minor,
            revision,
        }
    }

    fn from_read<R: Read + Seek>(data: &mut R) -> Result<Self> {
        Ok(Self {
            version: data.read_i32::<LE>()?,
            major: data.read_i32::<LE>()?,
            minor: data.read_i32::<LE>()?,
            revision: data.read_i32::<LE>()?,
        })
    }
}

impl fmt::Display for GodotVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.version, self.major, self.minor, self.revision
        )
    }
}

#[derive(Debug)]
struct FileMetaData {
    path: String,
    offset: u64,
    size: usize,
    md5: [u8; MD5_BYTES],
}

impl FileMetaData {
    fn from_read<R: Read + Seek>(data: &mut R) -> Result<Self> {
        let path_length = data.read_i32::<LE>()? as usize;
        let path = read_string(data, path_length)?
            .trim_start_matches(RESOURCE_PATH_PREFIX)
            .trim_start_matches(ASSET_PACK_PREFIX)
            .to_owned();

        let offset = data.read_i64::<LE>()? as u64;
        let size = data.read_i64::<LE>()? as usize;

        let mut md5 = [0; MD5_BYTES];
        data.read_exact(&mut md5)?;

        Ok(Self {
            path,
            offset,
            size,
            md5,
        })
    }
}

#[derive(Debug, Deserialize)]
struct PackMeta {
    name: String,
    id: String,
    version: String,
    author: String,
}

impl PackMeta {
    fn from_read<R: Read + Seek>(data: &mut R, file_meta: &FileMetaData) -> Result<Self> {
        data.seek(SeekFrom::Start(file_meta.offset))?;

        let json_string = read_string(data, file_meta.size)?;
        let info: Self = serde_json::from_str(json_string.as_str())?;

        Ok(info)
    }
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

    pack_file.seek(SeekFrom::Start(ASSET_PACK_MAGIC_FILE_HEADER.len() as u64))?;

    let pack = AssetPack::from_read(&mut pack_file).context("Could not read metadata")?;

    info!("Godot package version: {}", pack.godot_version);
    info!("Files in package: {}", pack.files_meta.len());

    info!("Pack name: {}", pack.meta.name);
    info!("Pack author: {}", pack.meta.author);
    info!("Pack version: {}", pack.meta.version);
    info!("Pack id: {}", pack.meta.id);

    Ok(pack)
}

fn is_root_json_file(path: &PathBuf) -> bool {
    path.extension().unwrap_or(OsStr::new("")) == OsStr::new("json")
        && path.parent() == Some(Path::new(""))
}

fn unpack_file(read: &mut dyn Read, file_size: usize) -> Result<Vec<u8>> {
    let mut file_data = vec![0; file_size];
    read.read_exact(file_data.as_mut_slice())?;

    Ok(file_data)
}

fn write_file(data: &Vec<u8>, file_path: &PathBuf) -> Result<()> {
    let folder = file_path.parent().unwrap();
    std::fs::create_dir_all(folder)?;

    let mut file = File::create(file_path)?;
    file.write_all(data.as_slice())?;

    Ok(())
}

fn is_dir_empty(dir: &PathBuf) -> bool {
    match dir.read_dir() {
        Ok(mut iter) => iter.next().is_none(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod test {
    use crate::asset_pack::{
        is_dir_empty, is_file_asset_pack, is_root_json_file, read_asset_pack, AssetPack,
        FileMetaData, GodotVersion, GODOT_METADATA_RESERVED_SPACE, MD5_BYTES,
    };
    use byteorder::{WriteBytesExt, LE};
    use log::LevelFilter;
    use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use tempfile::tempdir;

    const EXAMPLE_PACK: &str = "test_files/example_pack.dungeondraft_pack";
    const NOT_A_PACK: &str = "test_files/not_a_pack.txt";

    #[test]
    fn is_asset_pack_with_example_pack() {
        assert!(is_file_asset_pack(&PathBuf::from(EXAMPLE_PACK)).unwrap());
    }

    #[test]
    fn is_asset_pack_with_not_a_pack() {
        assert!(!is_file_asset_pack(&PathBuf::from(NOT_A_PACK)).unwrap());
    }

    #[test]
    fn is_dir_empty_empty_dir() {
        let temp = tempdir().unwrap();

        assert!(is_dir_empty(&temp.into_path()));
    }

    #[test]
    fn is_dir_empty_full_dir() {
        let temp = tempdir().unwrap();

        let file = temp.path().join("dir_not_empty.txt");
        std::fs::File::create(file).unwrap();

        assert!(!is_dir_empty(&temp.into_path()));
    }

    #[test]
    fn read_asset_pack_happy_flow() {
        TermLogger::init(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .unwrap();

        let pack = read_asset_pack(&PathBuf::from(EXAMPLE_PACK)).unwrap();

        let expected_id = "8UWKyQPf";

        assert_eq!(pack.godot_version, GodotVersion::new(1, 3, 2, 1));
        assert_eq!(pack.meta.name, "example_pack");
        assert_eq!(pack.meta.author, "megasploot");
        assert_eq!(pack.meta.id, expected_id);

        let expected_files = vec![
            "data/walls/sample_wall.dungeondraft_wall",
            "data/default.dungeondraft_tags",
            "data/tilesets/tileset_smart.dungeondraft_tileset",
            "data/tilesets/tileset_smart_double.dungeondraft_tileset",
            "data/tilesets/tileset_simple.dungeondraft_tileset",
            "pack.json",
            "textures/paths/sample_path.png",
            "textures/paths/streak.png",
            "textures/lights/sample_light.png",
            "textures/roofs/roof_name/edge.png",
            "textures/roofs/roof_name/hip.png",
            "textures/roofs/roof_name/tiles.png",
            "textures/roofs/roof_name/ridge.png",
            "textures/walls/sample_wall_end.png",
            "textures/walls/sample_wall.png",
            "textures/portals/sample_door.png",
            "textures/tilesets/smart/tileset_smart.png",
            "textures/tilesets/smart_double/tileset_smart_double.png",
            "textures/tilesets/simple/tileset_simple.png",
            "textures/objects/sample_barrel.png",
            "textures/objects/sample_cauldron.png",
        ];

        for file in expected_files {
            let mut path_with_id = String::from(expected_id);
            path_with_id.push('/');
            path_with_id.push_str(file);

            assert!(
                pack.files_meta.contains_key(&path_with_id),
                "Pack should contain file '{}', but does not",
                path_with_id
            );
        }
    }

    #[test]
    fn packed_file_from_read() {
        let path = String::from("res://test/bla.txt");
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
