#![cfg(test)]

use std::io::{Cursor, Write};

use anyhow::Result;
use byteorder::{WriteBytesExt, LE};

use crate::asset_pack::godot_version::GodotVersion;
use crate::asset_pack::AssetPack;

#[test]
fn asset_pack_from_read_happy_flow() {
    let raw_pack = create_raw_test_pack().unwrap();
    let mut cursor = Cursor::new(raw_pack);

    let pack = AssetPack::from_read(&mut cursor).unwrap();

    assert_eq!(pack.godot_version, GodotVersion::new(1, 3, 2, 4));
    assert_eq!(pack.meta.name, "example_pack");
    assert_eq!(pack.meta.id, "12345678");
    assert_eq!(pack.meta.author, "brass_phoenix");

    let color_overrides = pack.meta.custom_color_overrides;
    assert!(!color_overrides.enabled);
    assert_eq!(color_overrides.min_redness, 0.1);
    assert_eq!(color_overrides.min_saturation, 0.0);
    assert_eq!(color_overrides.red_tolerance, 0.04);

    assert!(pack
        .object_files
        .contains_key("textures/objects/random.png"));
    assert!(pack.other_files.contains_key("textures/portals/door.png"));

    let tags = pack.tags.tags;

    assert_eq!(tags.len(), 2);
    assert!(tags.contains_key("MyTag"));
    assert!(tags.contains_key("Colorable"));
    assert_eq!(tags.get("MyTag").unwrap().len(), 1);
    assert_eq!(tags.get("Colorable").unwrap().len(), 1);
    assert!(tags
        .get("MyTag")
        .unwrap()
        .contains("textures/objects/random.png"));
    assert!(tags
        .get("Colorable")
        .unwrap()
        .contains("textures/objects/sample_cauldron.png"));

    let tag_sets = pack.tags.sets;

    assert_eq!(tag_sets.len(), 1);
    assert!(tag_sets.contains_key("Example Set"));
    assert_eq!(tag_sets.get("Example Set").unwrap().len(), 1);
    assert!(tag_sets.get("Example Set").unwrap().contains("MyTag"));
}

#[test]
fn asset_pack_read_write_read_equivalence_check() {
    let raw_pack = create_raw_test_pack().unwrap();
    let mut cursor = Cursor::new(raw_pack);

    let pack = AssetPack::from_read(&mut cursor).unwrap();

    let mut written_pack = vec![];
    pack.to_write(&mut written_pack).unwrap();

    let mut re_read_cursor = Cursor::new(written_pack);
    let re_read_pack = AssetPack::from_read(&mut re_read_cursor).unwrap();

    assert_eq!(pack.godot_version, re_read_pack.godot_version);
    assert_eq!(pack.meta, re_read_pack.meta);

    assert_eq!(pack.object_files.len(), re_read_pack.object_files.len());
    assert_eq!(pack.other_files.len(), re_read_pack.other_files.len());

    assert_eq!(pack.tags, re_read_pack.tags);
}

fn create_raw_test_pack() -> Result<Vec<u8>> {
    let data: Vec<u8> = vec![];
    let mut cursor = Cursor::new(data);

    // Magic number indicating a Godot asset pack
    // "GDPC"
    let magic_num = [0x47, 0x44, 0x50, 0x43];
    cursor.write_all(&magic_num)?;

    // Godot version
    cursor.write_i32::<LE>(1)?; // Version
    cursor.write_i32::<LE>(3)?; // Major
    cursor.write_i32::<LE>(2)?; // Minor
    cursor.write_i32::<LE>(4)?; // Revision

    // Reserved space
    cursor.write_all(&[0; 16 * 4])?;

    // Number of files
    cursor.write_i32::<LE>(5)?;

    // ---- File metadata ----

    // For some reason Dungeondraft has two identical files in each pack.
    // One json file in the `packs` folder, and another in the `packs/<pack-id>` folder.
    write_file_meta(
        &mut cursor,
        "res://packs/12345678.json",
        468,
        TEST_PACK_META_JSON.len() as i64,
    )?;
    write_file_meta(
        &mut cursor,
        "res://packs/12345678/pack.json",
        683,
        TEST_PACK_META_JSON.len() as i64,
    )?;

    // Tag file
    write_file_meta(
        &mut cursor,
        "res://packs/12345678/data/default.dungeondraft_tags",
        898,
        TEST_PACK_TAGS_JSON.len() as i64,
    )?;

    // A random object file
    write_file_meta(
        &mut cursor,
        "res://packs/12345678/textures/objects/random.png",
        1080,
        TEST_PACK_FAKE_PNG.len() as i64,
    )?;

    // A random non-object file
    write_file_meta(
        &mut cursor,
        "res://packs/12345678/textures/portals/door.png",
        1090,
        TEST_PACK_FAKE_PNG.len() as i64,
    )?;

    // ---- File contents ----

    // res://packs/12345678.json
    cursor.write_all(TEST_PACK_META_JSON.as_bytes())?;
    // res://packs/12345678/pack.json
    cursor.write_all(TEST_PACK_META_JSON.as_bytes())?;
    // res://packs/12345678/data/default.dungeondraft_tags
    cursor.write_all(TEST_PACK_TAGS_JSON.as_bytes())?;

    // res://packs/12345678/textures/objects/random.png
    cursor.write_all(&TEST_PACK_FAKE_PNG)?;
    // res://packs/12345678/textures/portals/door.png
    cursor.write_all(&TEST_PACK_FAKE_PNG)?;

    Ok(cursor.into_inner())
}

fn write_file_meta(cursor: &mut Cursor<Vec<u8>>, path: &str, offset: i64, size: i64) -> Result<()> {
    cursor.write_i32::<LE>(path.len() as i32)?;
    cursor.write_all(path.as_bytes())?;
    cursor.write_i64::<LE>(offset)?;
    cursor.write_i64::<LE>(size)?;

    // md5 hash. Is actually unused in dungeondraft asset packs.
    cursor.write_all(&[0; 16])?;

    Ok(())
}

const TEST_PACK_META_JSON: &str = r#"
{
	"name": "example_pack",
	"id": "12345678",
	"version": "1",
	"author": "brass_phoenix",
	"custom_color_overrides": {
		"enabled": false,
		"min_redness": 0.1,
		"min_saturation": 0,
		"red_tolerance": 0.04
	}
}
"#;

const TEST_PACK_TAGS_JSON: &str = r#"
{
	"tags": {
		"MyTag": [
			"textures/objects/random.png"
		],
		"Colorable": [
			"textures/objects/sample_cauldron.png"
		]
	},
	"sets": {
		"Example Set": [
			"MyTag"
		]
	}
}
"#;

const TEST_PACK_FAKE_PNG: [u8; 10] = [0; 10];
