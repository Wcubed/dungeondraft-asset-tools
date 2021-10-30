use std::error::Error;
use std::io::Read;
use std::path::PathBuf;

const ASSET_PACK_MAGIC_FILE_HEADER: [u8; 4] = [0x47, 0x44, 0x50, 0x43];

fn is_file_asset_pack(pack: &PathBuf) -> Result<bool, Box<dyn Error>> {
    let mut file = std::fs::File::open(pack)?;

    let mut magic_file_number = [0; 4];
    file.read_exact(&mut magic_file_number)?;

    Ok(magic_file_number == ASSET_PACK_MAGIC_FILE_HEADER)
}

#[cfg(test)]
mod test {
    use crate::asset_pack::is_file_asset_pack;
    use std::path::PathBuf;

    #[test]
    fn is_asset_pack_with_example_pack() {
        assert!(
            is_file_asset_pack(&PathBuf::from("test_files/example_pack.dungeondraft_pack"))
                .unwrap()
        );
    }

    #[test]
    fn is_asset_pack_with_not_a_pack() {
        assert!(!is_file_asset_pack(&PathBuf::from("test_files/not_a_pack.txt")).unwrap());
    }
}
