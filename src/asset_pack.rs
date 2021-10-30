use anyhow::{anyhow, Context, Result};
use byteorder::{ReadBytesExt, LE};
use log::info;
use std::fmt;
use std::fmt::Formatter;
use std::io::Read;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::PathBuf;

const ASSET_PACK_MAGIC_FILE_HEADER: [u8; 4] = [0x47, 0x44, 0x50, 0x43];
const I32: i64 = 4;
const GODOT_METADATA_RESERVED_SPACE: i64 = 16 * I32;

#[derive(Debug)]
struct MetaData {
    version: GodotVersion,
    nr_of_files: u32,
}

impl MetaData {
    fn from_cursor(cursor: &mut Cursor<Vec<u8>>) -> Result<Self> {
        let version = GodotVersion::from_cursor(cursor)?;
        cursor.seek(SeekFrom::Current(GODOT_METADATA_RESERVED_SPACE))?;

        let nr_of_files = cursor.read_i32::<LE>()? as u32;

        Ok(Self {
            version,
            nr_of_files,
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
    fn from_cursor(cursor: &mut Cursor<Vec<u8>>) -> Result<Self> {
        Ok(Self {
            version: cursor.read_i32::<LE>()?,
            major: cursor.read_i32::<LE>()?,
            minor: cursor.read_i32::<LE>()?,
            revision: cursor.read_i32::<LE>()?,
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

fn is_file_asset_pack(pack: &PathBuf) -> Result<bool> {
    let mut file = std::fs::File::open(pack)?;

    let mut magic_file_number = [0; 4];
    file.read_exact(&mut magic_file_number)?;

    Ok(magic_file_number == ASSET_PACK_MAGIC_FILE_HEADER)
}

fn unpack_assets(pack: &PathBuf, destination: &PathBuf) -> Result<()> {
    if !is_dir_empty(destination) {
        return Err(anyhow!("Destination directory is not empty"));
    }

    let mut file = std::fs::File::open(pack).context("Asset pack file does not exist")?;

    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let mut cursor = Cursor::new(bytes);
    cursor.set_position(ASSET_PACK_MAGIC_FILE_HEADER.len() as u64);

    let metadata = MetaData::from_cursor(&mut cursor)?;

    info!("Godot package version: {}", metadata.version);

    println!("{:?}", metadata);

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
        is_dir_empty, is_file_asset_pack, unpack_assets, MetaData, GODOT_METADATA_RESERVED_SPACE,
    };
    use byteorder::{WriteBytesExt, LE};
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
        std::fs::File::create(file).unwrap();

        let result = unpack_assets(&PathBuf::from(EXAMPLE_PACK), &temp.into_path());
        assert!(result.is_err());
    }

    #[test]
    fn unpack_assets_happy_flow() {
        let temp = tempdir().unwrap();

        let result = unpack_assets(&PathBuf::from(EXAMPLE_PACK), &temp.into_path());
        assert!(result.is_ok());

        unimplemented!();
    }

    #[test]
    fn metadata_from_cursor() {
        let mut data = vec![];
        data.write_i32::<LE>(2).unwrap();
        data.write_i32::<LE>(1).unwrap();
        data.write_i32::<LE>(19).unwrap();
        data.write_i32::<LE>(12).unwrap();

        data.write_all(&[0; GODOT_METADATA_RESERVED_SPACE as usize])
            .unwrap();

        data.write_i32::<LE>(101).unwrap();

        let mut cursor = Cursor::new(data);
        let meta = MetaData::from_cursor(&mut cursor).unwrap();

        assert_eq!(meta.version.version, 2);
        assert_eq!(meta.version.major, 1);
        assert_eq!(meta.version.minor, 19);
        assert_eq!(meta.version.revision, 12);
        assert_eq!(meta.nr_of_files, 101);
    }
}
