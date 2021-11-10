use clap::{App, Arg};
use log::{error, info, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;

mod asset_pack;

fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

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
            Arg::with_name("OUTPUT_FILE")
                .help("Output file")
                .required(true)
                .index(2),
        )
        .get_matches();

    let path = PathBuf::from(matches.value_of("INPUT_FILE").unwrap());
    input_valid_or_exit(&path);

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            error!("Could not open input file '{}':\n{}", path.display(), e);
            exit(1)
        }
    };

    let pack = match asset_pack::AssetPack::from_read(&mut file) {
        Ok(p) => p,
        Err(e) => {
            error!(
                "Something went wrong while reading the asset pack '{}':\n{}",
                path.display(),
                e
            );
            exit(1)
        }
    };
}

fn input_valid_or_exit(path: &PathBuf) {
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
