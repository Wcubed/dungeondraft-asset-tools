use log::{info, LevelFilter};
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::io::Read;

mod asset_pack;
mod test_asset_pack;

fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    info!("Hello, world!");

    let mut file = std::fs::File::open("test_files/example_pack.dungeondraft_pack").unwrap();
    for val in file.bytes() {
        println!("{}", val.unwrap());
    }
}
