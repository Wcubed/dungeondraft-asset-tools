use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, LE};
use log::info;
use serde::Deserialize;
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
struct MetaData {
    version: GodotVersion,
    files_meta: Vec<FileMetaData>,
}

impl MetaData {
    fn from_read<R: Read + Seek>(data: &mut R) -> Result<Self> {
        let version = GodotVersion::from_read(data).context("Could not read godot version")?;
        data.read_exact(&mut [0; GODOT_METADATA_RESERVED_SPACE])?;

        let nr_of_files = data.read_i32::<LE>()? as usize;

        let mut files_meta = Vec::with_capacity(nr_of_files);
        for i in 0..nr_of_files {
            let file_meta = FileMetaData::from_read(data).context(format!(
                "Could not read file metadata of file {} from {}",
                i + 1,
                nr_of_files
            ))?;

            files_meta.push(file_meta);
        }

        Ok(Self {
            version,
            files_meta,
        })
    }
}

#[derive(Debug)]
struct GodotVersion {
    version: i32,
    major: i32,
    minor: i32,
    revision: i32,
}

impl GodotVersion {
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
        let path = read_string(data, path_length)?;

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
struct PackInfo {
    name: String,
    id: String,
    version: String,
    author: String,
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

pub fn unpack_assets(pack_path: &PathBuf, destination: &PathBuf) -> Result<()> {
    if !is_dir_empty(destination) {
        return Err(anyhow!("Destination directory is not empty"));
    }

    info!(
        "Unpacking {} into {}",
        pack_path.display(),
        destination.display()
    );

    let mut pack = std::fs::File::open(pack_path).context("Asset pack file does not exist")?;

    pack.seek(SeekFrom::Start(ASSET_PACK_MAGIC_FILE_HEADER.len() as u64))?;

    let metadata = MetaData::from_read(&mut pack).context("Could not read metadata")?;

    info!("Godot package version: {}", metadata.version);
    info!("Files in package: {}", metadata.files_meta.len());

    let mut maybe_info_file = None;

    for meta in metadata.files_meta {
        let file_path_without_prefixes = PathBuf::from(
            meta.path
                .trim_start_matches(RESOURCE_PATH_PREFIX)
                .trim_start_matches(ASSET_PACK_PREFIX),
        );

        let mut path = destination.clone();
        path.push(&file_path_without_prefixes);

        info!("Unpacking {}", path.display());

        // TODO: Check if file path does not exit the target directory.

        let data = unpack_file(&mut pack, meta.size)?;
        write_file(&data, &path)?;

        if is_root_json_file(&file_path_without_prefixes) {
            let json_string = std::str::from_utf8(&data)?;
            maybe_info_file = Some(serde_json::from_str::<PackInfo>(json_string)?);
        }
    }

    println!("{:?}", maybe_info_file);

    Ok(())
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
        is_dir_empty, is_file_asset_pack, is_root_json_file, unpack_assets, FileMetaData, MetaData,
        GODOT_METADATA_RESERVED_SPACE, MD5_BYTES,
    };
    use byteorder::{WriteBytesExt, LE};
    use log::LevelFilter;
    use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
    use std::fs::File;
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
    fn unpack_assets_refuse_to_unpack_in_non_empty_directory() {
        let temp = tempdir().unwrap();

        let file = temp.path().join("dir_not_empty.txt");
        File::create(file).unwrap();

        let result = unpack_assets(&PathBuf::from(EXAMPLE_PACK), &temp.into_path());
        assert!(result.is_err());
    }

    #[test]
    fn unpack_assets_happy_flow() {
        TermLogger::init(
            LevelFilter::Debug,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        )
        .unwrap();

        let temp = tempdir().unwrap();
        let temp_path = temp.into_path();

        unpack_assets(&PathBuf::from(EXAMPLE_PACK), &temp_path).unwrap();

        assert!(temp_path.join("8UWKyQPf.json").exists());

        let id = "8UWKyQPf";
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
            let path = temp_path.join(id).join(file);
            assert!(
                path.exists(),
                "Path '{}' should exist, but does not.",
                path.display()
            );
        }
    }

    #[test]
    fn metadata_from_read() {
        let mut data = vec![];
        data.write_i32::<LE>(2).unwrap();
        data.write_i32::<LE>(1).unwrap();
        data.write_i32::<LE>(19).unwrap();
        data.write_i32::<LE>(12).unwrap();

        data.write_all(&[0; GODOT_METADATA_RESERVED_SPACE as usize])
            .unwrap();

        let file_amount = 10;
        data.write_i32::<LE>(file_amount).unwrap();

        let path = String::from("res://something/something_else/filename.txt");
        let offset = 110;
        let size = 12;
        let md5 = [230; MD5_BYTES];

        for _ in 0..file_amount {
            data.write_i32::<LE>(path.len() as i32).unwrap();
            data.write_all(path.as_bytes()).unwrap();
            data.write_i64::<LE>(offset).unwrap();
            data.write_i64::<LE>(size).unwrap();
            data.write_all(&md5).unwrap();
        }

        let mut cursor = Cursor::new(data);
        let meta = MetaData::from_read(&mut cursor).unwrap();

        assert_eq!(meta.version.version, 2);
        assert_eq!(meta.version.major, 1);
        assert_eq!(meta.version.minor, 19);
        assert_eq!(meta.version.revision, 12);

        assert_eq!(meta.files_meta.len(), file_amount as usize);
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

        assert_eq!(file.path, path);
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
