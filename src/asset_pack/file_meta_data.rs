use log::trace;
use std::cmp::Ordering;
use std::io::{Read, Seek, Write};

use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use crate::asset_pack;
use crate::asset_pack::{ASSET_PACK_PREFIX, I32, I64, MD5_BYTES, RESOURCE_PATH_PREFIX};

#[derive(Debug, Clone)]
/// Comparing two `FileMetaData` will compare their offsets.
pub struct FileMetaData {
    pub path: String,
    pub offset: u64,
    pub size: usize,
    pub md5: [u8; MD5_BYTES],
}

impl FileMetaData {
    pub fn new(path: String, size: usize) -> Self {
        FileMetaData {
            path,
            offset: 0,
            size,
            md5: [0; MD5_BYTES],
        }
    }

    /// Strips `res://packs/<pack-id>/` if the file path starts with it.
    pub fn from_read<R: Read + Seek>(data: &mut R) -> anyhow::Result<Self> {
        let path_length = data.read_i32::<LE>()? as usize;
        let path_with_maybe_pack_id = asset_pack::read_string(data, path_length)?
            .trim_start_matches(RESOURCE_PATH_PREFIX)
            .trim_start_matches(ASSET_PACK_PREFIX)
            .to_owned();

        let (_id, path) = path_with_maybe_pack_id
            .split_once('/')
            .unwrap_or(("", path_with_maybe_pack_id.as_str()));

        trace!("File meta: {}", path);

        let offset = data.read_i64::<LE>()? as u64;
        let size = data.read_i64::<LE>()? as usize;

        let mut md5 = [0; MD5_BYTES];
        data.read_exact(&mut md5)?;

        Ok(Self {
            path: path.to_owned(),
            offset,
            size,
            md5,
        })
    }

    pub fn to_write<W: Write>(&self, data: &mut W) -> anyhow::Result<()> {
        data.write_i32::<LE>(self.path.len() as i32)?;
        data.write(self.path.as_bytes())?;
        data.write_i64::<LE>(self.offset as i64)?;
        data.write_i64::<LE>(self.size as i64)?;

        data.write_all(&[0; MD5_BYTES])?;

        Ok(())
    }

    pub fn calculate_binary_size(&self) -> usize {
        // An i32 to hold the string size.
        let mut size = I32;

        size += self.path.len();
        // Offset and file size
        size += I64 * 2;
        size += MD5_BYTES;

        size
    }
}

impl Eq for FileMetaData {}

impl PartialEq<Self> for FileMetaData {
    fn eq(&self, other: &Self) -> bool {
        self.offset.eq(&other.offset)
    }
}

impl PartialOrd<Self> for FileMetaData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.offset.partial_cmp(&other.offset)
    }
}

impl Ord for FileMetaData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.offset.cmp(&other.offset)
    }
}
