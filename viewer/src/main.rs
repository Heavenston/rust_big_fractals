#![feature(extract_if)]
#![allow(dead_code)]
use std::path::PathBuf;

use big_image_viewer::*;

use clap::Parser;

#[derive(clap::Args, Debug)]
pub struct AppSubcommand {
    #[arg(required = true, index = 1)]
    folder: PathBuf,
}

#[derive(clap::Args, Debug)]
pub struct ExtrapolateSubcommand {
    #[arg(required = true, index = 1)]
    folder: PathBuf,
    #[arg(short = 'p', long="format", default_value = "webp")]
    format: String,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    #[command(name = "start")]
    App(AppSubcommand),
    #[command(name = "extrapolate")]
    Extrapolate(ExtrapolateSubcommand),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long="debug", short='d')]
    debug: bool,

    #[command(subcommand)]
    start: Command,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    env_logger::builder()
        .filter(None, log::LevelFilter::Warn)
        .filter_module(
            "big_image_viewer",
            if args.debug
            { log::LevelFilter::Trace }
            else
            { log::LevelFilter::Info }
        )
        .init();

    match args.start {
        Command::App(app) => {
            app::start_app(&app.folder).await;
        },
        Command::Extrapolate(extr) => {
            format::utils::extrapolate_levels(
                extr.folder, &extr.format
            ).await;
        },
    }
}
