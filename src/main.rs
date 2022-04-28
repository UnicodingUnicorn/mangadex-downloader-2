#![feature(iter_intersperse)]
#[macro_use]
extern crate lazy_static;

mod api;
mod chapter;
mod manga;
mod range;
mod ratelimits;
mod requester;
mod types;
mod utils;

use api::{ API, APIError };
use chapter::ChapterMetadata;
use range::{ Range, RangeError };

use std::path::Path;

use clap::Parser;
use log::{ info, error };
use simplelog::{ self, TermLogger, LevelFilter, TerminalMode, ColorChoice };
use thiserror::Error;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Arguments {
    url: Option<String>,
    #[clap(short, long, default_value = "en")]
    language: String,
    #[clap(long)]
    metadata: bool,
    #[clap(short, long, default_value = "output")]
    output_dir: String,
    #[clap(short, long)]
    range: Option<String>,
    #[clap(short, long)]
    quiet: bool,
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    let log_level = match args.quiet {
        true => LevelFilter::Warn,
        false => LevelFilter::Info,
    };

    TermLogger::init(log_level, simplelog::Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();

    if args.url == None {
        error!("Manga url has not been specified");
        std::process::exit(1);
    }

    if let Err(e) = run(args).await {
        error!("{}", e);
        std::process::exit(1);
    }

    info!("Done!");
}

#[derive(Debug, Error)]
enum ProgramError {
    #[error("{0}")]
    API(#[from] APIError),
    #[error("{0}")]
    Range(#[from] RangeError),
    #[error("specified language is not available")]
    LanguageNotAvailable,
    #[error("no title is available")]
    TitleNotAvailable,
}

async fn run(args:Arguments) -> Result<(), ProgramError> {
    let url = args.url.unwrap();
    let mut api = API::new();

    info!("Retrieving metadata...");
    let manga_metadata = api.get_manga_metadata(&url).await?;
    if args.metadata {
        manga_metadata.print();
        return Ok(());
    }

    if !manga_metadata.languages.iter().any(|lang| lang == &args.language) {
        return Err(ProgramError::LanguageNotAvailable);
    }

    let ranges = args.range.map(|r| Range::from_str(&r))
        .transpose()?;

    let title = manga_metadata.get_title(&args.language).ok_or(ProgramError::TitleNotAvailable)?;

    info!("Retrieving chapter metadata...");
    let chapter_metadata = api.get_chapter_metadata(&manga_metadata, args.quiet).await?;
    let download_chapter_metadata = chapter_metadata.iter()
        .filter(|m| m.language == args.language)
        .filter(|m| ranges.as_ref().map(|r| r.iter().any(|range| range.in_range(&m.volume, &m.chapter))).unwrap_or(true))
        .map(|m| m.clone())
        .collect::<Vec<ChapterMetadata>>();

    info!("Retrieving chapter images download data...");
    let chapters = api.get_chapters(&download_chapter_metadata, args.quiet).await?;

    info!("Downloading chapters...");
    let master_directory = Path::new(&args.output_dir).join(Path::new(&title));
    api.download_chapters(&chapters, &master_directory, args.quiet).await?;

    Ok(())
}
