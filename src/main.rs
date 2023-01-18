#![feature(iter_intersperse)]
#[macro_use]
extern crate lazy_static;

mod api;
mod chapter;
mod coverart;
mod image;
mod manga;
mod metadata;
mod range;
mod ratelimits;
mod requester;
mod types;
mod utils;

use api::{ API, APIError };
use chapter::ChapterMetadata;
use coverart::CoverArt;
use metadata::{ Metadata, MetadataError };
use range::{ Range, RangeError };

use std::path::Path;

use clap::{ Parser, ValueEnum };
use log::{ info, error };
use simplelog::{ self, TermLogger, LevelFilter, TerminalMode, ColorChoice };
use thiserror::Error;

#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum MetadataOutputFormat {
    TOML,
    JSON,
}
impl MetadataOutputFormat {
    pub fn file_format(&self) -> &'static str {
        match self {
            MetadataOutputFormat::TOML => "toml",
            MetadataOutputFormat::JSON => "json",
        }
    }
}

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Arguments {
    /// The URL to the manga, e.g. https://mangadex.org/title/348966d0-c807-45cf-9260-8adf006a9da6/kono-bijutsubu-ni-wa-mondai-ga-aru
    url: Option<String>,
    #[clap(short, long, default_value = "en")]
    /// Preferred download language, in ISO-639 form, e.g. en
    language: String,
    #[clap(long)]
    /// Display metadata only; do not download
    metadata: bool,
    #[clap(short, long, default_value = "output")]
    /// Output directory. Manga will be created as a subfolder to this.
    output_dir: String,
    #[clap(short, long)]
    /// Chapter range to download, leave blank to download the whole manga.
    range: Option<String>,
    #[clap(short, long)]
    /// Suppress all terminal output
    quiet: bool,
    #[clap(long, default_values=&["ja-ro", "ja", "en"])]
    /// Title languages to download into metadata file, in ISO-639 form. Set to 'all' to download all titles.
    metadata_title_languages: Vec<String>,
    #[clap(long, value_enum, default_value_t=MetadataOutputFormat::TOML)]
    /// Metadata output file format.
    metadata_file_format: MetadataOutputFormat,
    #[clap(long)]
    /// Don't save metadata
    no_metadata: bool,
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
    #[error("{0}")]
    Metadata(#[from] MetadataError),
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

    info!("Retrieving cover art metadata...");
    let cover_art_metadata = api.get_cover_art(&manga_metadata.id, args.quiet).await?;

    info!("Retrieving chapter images download data...");
    let download_chapter_metadata = chapter_metadata.iter()
        .filter(|m| m.language == args.language)
        .filter(|m| ranges.as_ref().map(|r| r.iter().any(|range| range.in_range(&m.volume, &m.chapter))).unwrap_or(true))
        .map(|m| m.clone())
        .collect::<Vec<ChapterMetadata>>();
    let chapters = api.get_chapters(&download_chapter_metadata, args.quiet).await?;

    info!("Downloading chapters...");
    let master_directory = Path::new(&args.output_dir).join(Path::new(&utils::escape_path(&title)));
    api.download_chapters(&chapters, &master_directory, args.quiet).await?;

    info!("Downloading cover art...");
    let download_cover_arts = cover_art_metadata.iter()
        .filter(|cam| ranges.as_ref().map(|r| r.iter().any(|range| range.in_volume_range(&cam.volume))).unwrap_or(true))
        .map(|cam| cam.clone())
        .collect::<Vec<CoverArt>>();
    api.download_cover_art(&download_cover_arts, &master_directory, args.quiet).await?;

    if !args.no_metadata {
        info!("Saving metadata...");
        let metadata = Metadata::new(&manga_metadata, &args.language, &args.metadata_title_languages);
        metadata.save(&master_directory, args.metadata_file_format)?;
    }

    Ok(())
}
