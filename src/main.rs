use crate::asset_pack::AssetPack;
use clap::{App, Arg};
use log::{debug, error, info, LevelFilter};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;

mod asset_pack;

fn main() {
    let matches = App::new("Dungeondraft Asset Tools")
        .version("0.1")
        .author("Wybe Westra <dev@wwestra.nl>")
        .about("For now can remove empty tags and tag groups from Dungeondraft asset packs.")
        .arg(
            Arg::with_name("INPUT_FILE")
                .help("Input file")
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

    let input_path = PathBuf::from(matches.value_of("INPUT_FILE").unwrap());
    input_path_valid_or_exit(&input_path);

    let mut output_path = PathBuf::from(matches.value_of("OUTPUT_DIR").unwrap());
    output_path_valid_or_exit(&input_path, &output_path);

    let overwrite_allowed = matches.is_present("force_overwrite");

    let mut pack = read_pack_or_exit(&input_path);

    info!("Godot package version: {}", pack.godot_version);
    info!("Files in package: {}", pack.other_files.len());

    info!("Pack name: {}", pack.meta.name);
    info!("Pack author: {}", pack.meta.author);
    info!("Pack version: {}", pack.meta.version);
    info!("Pack id: {}", pack.meta.id);

    debug!("{}", pack.tags);

    pack.clean_tags();

    debug!("Tags after cleaning: {}", pack.tags);

    output_path.push(input_path.file_name().unwrap());
    info!("Saving to '{}", output_path.display());

    if let Err(e) = fs::create_dir_all(output_path.parent().unwrap()) {
        error!("Could not create the necessary directories:\n{}", e);
    }

    if output_path.exists() {
        if overwrite_allowed {
            info!(
                "Overwriting already existing file '{}'.",
                output_path.display()
            )
        } else {
            error!(
                "Output file '{}' already exists. If you want to overwrite, call again with the `-F` argument.",
                output_path.display()
            );
            exit(1);
        }
    }

    let mut file = match File::create(&output_path) {
        Ok(f) => f,
        Err(e) => {
            error!(
                "Could not create the output file '{}':\n{}",
                output_path.display(),
                e
            );
            exit(1);
        }
    };

    match pack.to_write(&mut file) {
        Ok(_) => {}
        Err(e) => {
            error!(
                "Something went wrong while writing the pack file '{}':\n{}",
                output_path.display(),
                e
            );
            exit(1);
        }
    }

    info!("Done");
}

fn output_path_valid_or_exit(input_path: &PathBuf, output_path: &PathBuf) {
    if input_path.exists() && output_path.exists() {
        let canonical_input_parent = input_path.parent().unwrap().canonicalize().unwrap();
        let canonical_output = output_path.canonicalize().unwrap();

        if canonical_output == canonical_input_parent {
            error!("The output directory and input directory are the same: '{}' refusing to overwrite pack files.", canonical_output.display());
            exit(1);
        }
    }
}

fn input_path_valid_or_exit(path: &PathBuf) {
    if !path.exists() {
        error!("Input file '{}' does not exist.", path.display());
        exit(1);
    }
    match asset_pack::is_file_asset_pack(&path) {
        Ok(false) => {
            error!(
                "Input file '{}' is not a dungeondraft asset pack.",
                path.display()
            );
            exit(1);
        }
        Ok(true) => {}
        Err(e) => error!(
            "Something went wrong while reading the asset pack '{}':\n{}",
            path.display(),
            e
        ),
    }
}

fn read_pack_or_exit(path: &PathBuf) -> AssetPack {
    info!("Reading pack file '{}'", path.display());

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            error!("Could not open input file '{}':\n{}", path.display(), e);
            exit(1)
        }
    };

    match asset_pack::AssetPack::from_read(&mut file) {
        Ok(p) => p,
        Err(e) => {
            error!(
                "Something went wrong while reading the asset pack '{}':\n{}",
                path.display(),
                e
            );
            exit(1)
        }
    }
}
