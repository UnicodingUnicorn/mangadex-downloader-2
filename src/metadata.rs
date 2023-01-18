use crate::MetadataOutputFormat;
use crate::manga::MangaMetadata;
use serde::{ Deserialize, Serialize };
use thiserror::Error;

use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("error writing metadata file: {0}")]
    IO(#[from] std::io::Error),
    #[error("error serialising metadata to toml: {0}")]
    TOMLSerialisation(#[from] toml::ser::Error),
    #[error("error serialising metadata to json: {0}")]
    JSONSerialisation(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Metadata {
    title: String,
    alt_titles: Vec<String>,
    description: String,
    tags: Vec<String>,
}
impl Metadata {
    pub fn new(metadata:&MangaMetadata, preferred_language:&str, metadata_title_languages:&[String]) -> Self {
        let alt_titles = match metadata_title_languages.iter().any(|o| o == "all") {
            true => metadata.alt_titles.iter()
                .map(|(_, v)| v.iter())
                .flatten()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            false => metadata.alt_titles.iter()
                .filter(|(k, _)| metadata_title_languages.iter().any(|l| l == *k))
                .map(|(_, v)| v.iter())
                .flatten()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        };

        let tags = metadata.tags.iter()
            .filter_map(|t| t.get(preferred_language))
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        Self {
            title: metadata.get_title(preferred_language).unwrap_or(String::new()),
            alt_titles,
            description: metadata.get_description(preferred_language).unwrap_or(String::new()),
            tags,
        }
    }

    pub fn save(&self, master_directory:&Path, format:MetadataOutputFormat) -> Result<(), MetadataError> {
        let data = match format {
            MetadataOutputFormat::TOML => toml::to_string(self)?,
            MetadataOutputFormat::JSON => serde_json::to_string(self)?,
        };

        let mut file = File::create(master_directory.join(Path::new(&format!("metadata.{}", format.file_format()))))?;
        file.write_all(data.as_bytes())?;

        Ok(())
    }
}
