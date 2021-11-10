use crate::asset_pack::AssetPack;
use anyhow::{Context, Result};
use clap::{App, Arg};
use glob::glob;
use log::{debug, error, info, warn, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;

mod asset_pack;

const ASSET_PACK_EXTENSION: &str = ".dungeondraft_pack";

fn main() {
    let matches = App::new("Dungeondraft Asset Tools")
        .version("0.1")
        .author("Wybe Westra <dev@wwestra.nl>")
        .about("For now can remove empty tags and tag groups from Dungeondraft asset packs.")
        .arg(
            Arg::with_name("INPUT_DIR")
                .help("Input directory, will scan recursively for `*.dungeondraft_pack` files")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("OUTPUT_DIR")
                .help(
                    "The resulting asset pack will be placed in this directory.\n\
                Should not be the same as the directory of the input file.",
                )
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("force_overwrite")
                .short("F")
                .help("Overwrite existing output files"),
        )
        .arg(Arg::with_name("v").short("v").help("Print extra info"))
        .get_matches();

    let verbosity = if matches.is_present("v") {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    TermLogger::init(
        verbosity,
        ConfigBuilder::default()
            .set_thread_level(LevelFilter::Trace)
            .set_target_level(LevelFilter::Trace)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let input_dir = PathBuf::from(matches.value_of("INPUT_DIR").unwrap());
    input_dir_valid_or_exit(&input_dir);

    let output_dir = PathBuf::from(matches.value_of("OUTPUT_DIR").unwrap());
    output_dir_valid_or_exit(&input_dir, &output_dir);

    let overwrite_allowed = matches.is_present("force_overwrite");

    if let Err(e) = fs::create_dir_all(output_dir.parent().unwrap()) {
        error!("Could not create the output directory:\n{}", e);
    }

    let input_glob = String::new() + input_dir.to_str().unwrap() + "/**/*" + ASSET_PACK_EXTENSION;

    for entry in glob(&input_glob).expect("Glob pattern could not be parsed") {
        match entry {
            Ok(path) => {
                info!("{}", path.display());
                handle_pack(&path, &output_dir, overwrite_allowed);
            }
            Err(e) => warn!("{}", e),
        }
    }

    info!("Done");
}

fn output_dir_valid_or_exit(input_dir: &PathBuf, output_dir: &PathBuf) {
    if input_dir.exists() && output_dir.exists() {
        let canonical_input = input_dir.canonicalize().unwrap();
        let canonical_output = output_dir.canonicalize().unwrap();

        if canonical_output == canonical_input {
            error!(
                "The output directory and input directory are the same: '{}'.",
                canonical_output.display()
            );
            exit(1);
        }
    }
}

fn input_dir_valid_or_exit(input_dir: &PathBuf) {
    if !input_dir.exists() {
        error!("Input directory '{}' does not exist.", input_dir.display());
        exit(1);
    }
}

fn handle_pack(pack_path: &PathBuf, output_dir: &PathBuf, overwrite_allowed: bool) {
    let mut pack = match read_pack(&pack_path) {
        Ok(p) => p,
        Err(e) => {
            warn!("Could not read packfile '{}':\n{}", pack_path.display(), e);
            return;
        }
    };

    info!("Godot package version: {}", pack.godot_version);
    info!("Files in package: {}", pack.other_files.len());

    info!("Pack name: {}", pack.meta.name);
    info!("Pack author: {}", pack.meta.author);
    info!("Pack version: {}", pack.meta.version);
    info!("Pack id: {}", pack.meta.id);

    debug!("{}", pack.tags);

    pack.clean_tags();

    debug!("After cleaning\n{}", pack.tags);

    let mut output_path = output_dir.clone();
    output_path.push(pack_path.file_name().unwrap());

    write_pack(&pack, &output_path, overwrite_allowed);
}

fn read_pack(path: &PathBuf) -> Result<AssetPack> {
    info!("Reading pack file '{}'", path.display());

    let mut file =
        File::open(&path).context(format!("Could not open pack file '{}'", path.display()))?;

    asset_pack::AssetPack::from_read(&mut file)
}

fn write_pack(pack: &AssetPack, output_path: &PathBuf, overwrite_allowed: bool) {
    info!(
        "Saving pack '{}' to '{}",
        pack.meta.name,
        output_path.display()
    );

    if output_path.exists() {
        if overwrite_allowed {
            info!("Overwriting '{}'.", output_path.display())
        } else {
            warn!(
                "Output file '{}' already exists. If you want to overwrite, call again with the `-F` argument.",
                output_path.display()
            );
            return;
        }
    }

    let mut file = match File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            warn!(
                "Could not create the output file '{}':\n{}",
                output_path.display(),
                e
            );
            return;
        }
    };

    match pack.to_write(&mut file) {
        Ok(_) => {}
        Err(e) => {
            warn!(
                "Something went wrong while writing the pack file '{}':\n{}",
                output_path.display(),
                e
            );
        }
    }
}
