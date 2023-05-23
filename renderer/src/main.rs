#![feature(int_roundings)]
pub mod shader_prep;
pub mod renderer;
use renderer::*;

use std::path::PathBuf;
use anyhow::anyhow;
use clap::{ value_parser, Parser };

fn position_arg_parse(s: &str) -> anyhow::Result<(u32, u32)> {
    let (a, b) = s.split_once('x').ok_or(anyhow!("Syntax is 'NUMxNUM'"))?;
    Ok((a.parse()?, b.parse()?))
}

/// Render fractals potentially in sections !
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the wgsl shader used for rendering
    #[arg(required = true, index = 1, value_name = "shader")]
    shader: PathBuf,
    /// Path to the folder where to save the rendered images
    #[arg(long="out", short='o', value_name = "out_folder", default_value = ".")]
    out_folder: PathBuf,
    /// Extension of the output images
    #[arg(long="format", short='p', value_name = "format", default_value = "bmp")]
    format: String,

    /// Size of the rendered images
    #[arg(long="size", default_value_t = 2048)]
    size: u32,
    /// If specified the images will be resized before being saved
    #[arg(long="resize")]
    resize: Option<u32>,
    /// How many subdivisions on each dimensions should be renderer
    /// (5 subdivisions means there will be 5x5=25 images total)
    #[arg(
        long="subdivides", short='s', default_value_t = 1,
        value_parser = value_parser!(u32).range(1..)
    )]
    subdivisions: u32,

    #[arg(long="from", short='f', value_parser = position_arg_parse, default_value = "0x0")]
    from: (u32, u32),
    #[arg(long="to", short='t', value_parser = position_arg_parse)]
    to: Option<(u32, u32)>,

    /// Enable debug output
    #[arg(long="debug", short='d')]
    debug: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let to = args.to.unwrap_or((args.subdivisions - 1, args.subdivisions - 1));

    env_logger::builder()
        .filter(None, log::LevelFilter::Warn)
        .filter_module(
            "fractals",
            if args.debug
            { log::LevelFilter::Trace }
            else
            { log::LevelFilter::Info }
        )
        .init();

    log::info!("Using render size:  {:?}", args.size);
    log::info!("Using resuze size:  {:?}", args.resize);
    log::info!("Using subdivisions: {:?}", args.subdivisions);
    log::info!(
        "Rendering sections: {}x{} to {}x{}",
        args.from.0, args.from.1, to.0, to.1
    );
    log::info!("Using shader:       {:?}", args.shader);
    log::info!("Using output folder:{:?}", args.out_folder);

    log::debug!("Creating renderer");
    let renderer = Renderer::new(args.size, args.shader).await;
    log::debug!("Created");

    std::fs::create_dir_all(&args.out_folder).unwrap();

    let mut set = tokio::task::JoinSet::new();

    let resize = args.resize;
    let subdivisions = args.subdivisions;
    for sx in args.from.0..=to.0 {
        for sy in args.from.1..=to.1 {
            log::info!("Rendering {sx}x{sy}...");
            let s1 = renderer.render_section(SectionInfo {
                subdivisions: args.subdivisions,
                subdiv_pos: (sx, sy),
            }).await;
            let out_folder = args.out_folder.clone();
            let format = args.format.clone();
            set.spawn_blocking(move || {
                let ns1 =
                    if let Some(ns) = resize {
                        log::debug!("Resizing {sx}x{sy}...");
                        image::imageops::resize(&s1, ns, ns, image::imageops::FilterType::Lanczos3)
                    } else { s1 };
                log::debug!("Saving {sx}x{sy}...");
                ns1.save(
                    out_folder.join(&format!("{subdivisions}_{sx}x{sy}.{format}"))
                ).unwrap();
                log::debug!("Finished {sx}x{sy}");
            });
        }
    }

    while let Some(x) = set.join_next().await {
        x.unwrap();
    }
}
