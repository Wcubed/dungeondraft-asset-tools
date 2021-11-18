use anyhow::{Context, Result};
use log::info;
use std::io::Read;

pub const ASSET_PACK_MAGIC_FILE_HEADER: [u8; 4] = [0x47, 0x44, 0x50, 0x43];
pub const I32: usize = 4;
pub const I64: usize = 8;
pub const GODOT_METADATA_RESERVED_SPACE: usize = 16 * I32;
pub const MD5_BYTES: usize = 16;

pub fn read_string(data: &mut dyn Read, length: usize) -> Result<String> {
    let mut bytes = vec![0; length];
    data.read_exact(bytes.as_mut_slice())
        .context("Could not read string")?;

    Ok(String::from_utf8(bytes).context("Could not convert string from bytes")?)
}

pub fn display_file_as_info(file_data: &str) {
    info!("```\n{}\n```", file_data);
}
