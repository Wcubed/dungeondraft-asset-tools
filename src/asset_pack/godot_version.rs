use std::fmt;
use std::fmt::Formatter;
use std::io::{Read, Seek, Write};

use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use crate::asset_pack::I32;

#[derive(Debug, Eq, PartialEq)]
pub struct GodotVersion {
    version: i32,
    major: i32,
    minor: i32,
    revision: i32,
}

impl GodotVersion {
    pub fn new(version: i32, major: i32, minor: i32, revision: i32) -> Self {
        Self {
            version,
            major,
            minor,
            revision,
        }
    }

    pub fn from_read<R: Read + Seek>(data: &mut R) -> anyhow::Result<Self> {
        Ok(Self {
            version: data.read_i32::<LE>()?,
            major: data.read_i32::<LE>()?,
            minor: data.read_i32::<LE>()?,
            revision: data.read_i32::<LE>()?,
        })
    }

    pub fn to_write<W: Write>(&self, data: &mut W) -> anyhow::Result<()> {
        data.write_i32::<LE>(self.version)?;
        data.write_i32::<LE>(self.major)?;
        data.write_i32::<LE>(self.minor)?;
        data.write_i32::<LE>(self.revision)?;

        Ok(())
    }

    pub fn size_in_bytes() -> usize {
        I32 * 4
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
