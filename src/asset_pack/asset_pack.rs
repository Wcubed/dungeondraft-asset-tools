use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use anyhow::{bail, Context};
use byteorder::{ReadBytesExt, WriteBytesExt, LE};

use crate::asset_pack::file_meta_data::FileMetaData;
use crate::asset_pack::godot_version::GodotVersion;
use crate::asset_pack::pack_meta::PackMeta;
use crate::asset_pack::path_utils::*;
use crate::asset_pack::tags::Tags;
use crate::asset_pack::utils::*;

#[derive(Debug)]
pub struct AssetPack {
    pub godot_version: GodotVersion,
    pub meta: PackMeta,
    pub tags: Tags,
    pub object_files: HashMap<String, Vec<u8>>,
    pub other_files: HashMap<String, Vec<u8>>,
}

impl AssetPack {
    pub fn from_read<R: Read + Seek>(data: &mut R) -> anyhow::Result<Self> {
        let mut magic_file_number = [0; 4];
        data.read_exact(&mut magic_file_number)?;

        if magic_file_number != ASSET_PACK_MAGIC_FILE_HEADER {
            warn!(
                "First bytes of file do not indicate this is an asset pack. \
            Reading might not work correctly, attempting anyway."
            );
        }

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
                let data_string = String::from_utf8(file_data)?;

                maybe_meta = match json5::from_str(&data_string) {
                    Ok(meta) => Some(meta),
                    Err(e) => {
                        display_file_as_info(&data_string);
                        bail!("Could not parse pack metadata file:\n{}", e)
                    }
                };
            } else if is_tags_file(&meta.path) {
                let data_string = String::from_utf8(file_data)?;

                maybe_tags = match json5::from_str(&data_string) {
                    Ok(tags) => Some(tags),
                    Err(e) => {
                        display_file_as_info(&data_string);
                        bail!("Could not parse object tags file:\n{}", e)
                    }
                };
            } else if is_objects_file(&meta.path) {
                object_files.insert(meta.path.clone(), file_data);
            } else if !is_pack_file(pathbuf) {
                other_files.insert(meta.path.clone(), file_data);
            }
        }

        // Some packs don't include any object files, and therefore also don't have a tags file.
        let tags = maybe_tags.unwrap_or(Tags::new());

        Ok(AssetPack {
            godot_version,
            meta: maybe_meta.unwrap(),
            tags,
            object_files,
            other_files,
        })
    }

    pub fn to_write<W: Write>(&self, data: &mut W) -> anyhow::Result<()> {
        data.write_all(&ASSET_PACK_MAGIC_FILE_HEADER)?;
        self.godot_version.to_write(data)?;
        data.write_all(&[0; GODOT_METADATA_RESERVED_SPACE])?;

        let file_path_prefix =
            RESOURCE_PATH_PREFIX.to_owned() + ASSET_PACK_PREFIX + self.meta.id.as_str();

        let pack_meta_file = json5::to_string(&self.meta)?.as_bytes().to_vec();
        let root_pack_file_metadata =
            FileMetaData::new(file_path_prefix.clone() + ".json", pack_meta_file.len());
        let pack_file_metadata = FileMetaData::new(
            file_path_prefix.clone() + "/" + PACK_FILE_NAME,
            pack_meta_file.len(),
        );

        let tags_file = json5::to_string(&self.tags)?.as_bytes().to_vec();
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

    /// Does the following operations, in the given order:
    /// - Removes non-existing objects from tags.
    /// - Removes empty tags.
    /// - Removes non existing tags from tag sets.
    /// - Removes empty tag sets.
    pub fn clean_tags(&mut self) {
        info!("Cleaning empty tags and tag groups.");

        let mut empty_tags = vec![];

        for (tag, files) in self.tags.tags.iter_mut() {
            let mut not_existing_files = vec![];

            for file in files.iter() {
                if !self.object_files.contains_key(file) {
                    not_existing_files.push(file.clone());
                }
            }

            for file in not_existing_files {
                debug!(
                    "Removing file '{}' from tag '{}' because it does not exist.",
                    file, tag
                );
                files.remove(&file);
            }

            if files.is_empty() {
                empty_tags.push(tag.clone());
            }
        }

        for tag in empty_tags.iter() {
            debug!("Removing tag '{}' because it is empty.", tag);
            self.tags.tags.remove(tag);
        }

        let mut empty_sets = vec![];

        for (set, tags) in self.tags.sets.iter_mut() {
            let mut not_existing_tags = vec![];

            for tag in tags.iter() {
                if !self.tags.tags.contains_key(tag) {
                    not_existing_tags.push(tag.clone());
                }
            }

            for tag in not_existing_tags {
                debug!(
                    "Removing tag '{}' from set '{}' because it does not exist.",
                    tag, set
                );
                tags.remove(&tag);
            }

            if tags.is_empty() {
                empty_sets.push(set.clone());
            }
        }

        for set in empty_sets.iter() {
            debug!("Removing set '{}' because it is empty.", set);
            self.tags.sets.remove(set);
        }

        info!(
            "Removed {} empty tags, and {} empty tag sets.",
            empty_tags.len(),
            empty_sets.len()
        );
    }

    fn get_files_in_tag(&self, tag: &str) -> Option<&HashSet<String>> {
        self.tags.tags.get(tag)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;
    use std::io::{Cursor, Write};
    use std::iter::FromIterator;
    use std::path::PathBuf;

    use byteorder::{WriteBytesExt, LE};

    use crate::asset_pack::asset_pack::*;
    use crate::asset_pack::color_overrides::ColorOverrides;

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

    #[test]
    fn test_clean_tags() {
        let rock_file = "textures/objects/rock.png".to_string();

        let mut pack = new_empty_pack();
        pack.tags.tags.insert(
            "rocks".to_string(),
            HashSet::from_iter(vec![rock_file.clone(), "does_not_exist.jpg".to_string()]),
        );
        pack.tags.tags.insert("empty".to_string(), HashSet::new());

        pack.tags.sets.insert("empty".to_string(), HashSet::new());
        pack.tags.sets.insert(
            "will_be_empty".to_string(),
            HashSet::from_iter(vec!["empty".to_string()]),
        );
        pack.tags.sets.insert(
            "loses_one_tag".to_string(),
            HashSet::from_iter(vec!["rocks".to_string(), "empty".to_string()]),
        );

        pack.object_files.insert(rock_file.clone(), vec![]);

        pack.clean_tags();

        assert!(!pack.tags.tags.contains_key("empty"));
        assert!(pack.tags.tags.contains_key("rocks"));
        let rock_tags = pack.get_files_in_tag("rocks").unwrap();

        assert_eq!(rock_tags.len(), 1);
        assert!(rock_tags.contains(&rock_file));

        assert_eq!(pack.tags.sets.len(), 1);
        assert!(pack.tags.sets.contains_key("loses_one_tag"));

        let one_tag_set = pack.tags.sets.get("loses_one_tag").unwrap();
        assert_eq!(one_tag_set.len(), 1);
        assert!(one_tag_set.contains("rocks"));
    }

    fn new_empty_pack() -> AssetPack {
        AssetPack {
            godot_version: GodotVersion::new(0, 0, 0, 0),
            meta: PackMeta {
                name: "".to_string(),
                id: "".to_string(),
                version: "".to_string(),
                author: "".to_string(),
                custom_color_overrides: Some(ColorOverrides {
                    enabled: false,
                    min_redness: 0.0,
                    min_saturation: 0.0,
                    red_tolerance: 0.0,
                }),
            },
            tags: Tags {
                tags: Default::default(),
                sets: Default::default(),
            },
            object_files: Default::default(),
            other_files: Default::default(),
        }
    }
}
