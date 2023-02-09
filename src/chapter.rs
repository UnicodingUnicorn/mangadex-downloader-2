use crate::image::Image;
use crate::range::Range;
use crate::requester::{ RateLimitedRequester, RequesterError };
use crate::types::{ ChapterData, ChapterImageResponse };
use crate::utils;

use std::collections::HashMap;
use std::path::Path;
use std::fs::{ self, File };
use std::io::Write;
use std::time::Duration;

use pbr::ProgressBar;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ChapterMetadata {
    pub id:String,
    pub volume: String,
    pub chapter: String,
    pub language: String,
    pub group: String,
}
impl ChapterMetadata {
    pub fn from_chapter_data(raw:ChapterData) -> Option<Self> {
        let volume = match &raw.attributes.volume {
            Some(volume) => volume.clone(),
            None => String::new(),
        };

        let chapter = match &raw.attributes.chapter {
            Some(chapter) => chapter.clone(),
            None => String::new(),
        };

        let group = raw.relationships.iter()
            .filter(|r| r.id == "scanlation_group")
            .filter_map(|r| Some(r.attributes.as_ref()?.name.as_ref()?.clone()))
            .next().unwrap_or(String::new());

        Some(Self {
            id: raw.id,
            volume,
            chapter,
            language: raw.attributes.language?,
            group,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ChapterMetadataSeries {
    max_tl: String,
    max_tl_count: u64,
    tl_group_counts: HashMap<String, u64>,
    chapters: Vec<ChapterMetadata>,
}
impl ChapterMetadataSeries {
    pub fn new(raw:Vec<ChapterData>) -> Self {
        let mut cms = Self {
            max_tl: String::new(),
            max_tl_count: 0,
            tl_group_counts: HashMap::new(),
            chapters: Vec::new(),
        };

        cms.add_metadata(raw);

        cms
    }

    pub fn add_metadata(&mut self, mut raw:Vec<ChapterData>) {
        // Convert new chapters
        let mut chapters = raw.drain(..)
            .filter_map(|r| ChapterMetadata::from_chapter_data(r))
            .collect::<Vec<ChapterMetadata>>();

        // Recalculate TL-groups
        // TODO: Less clones
        for chapter in chapters.iter() {
            let n = match self.tl_group_counts.get(&chapter.group) {
                Some(n) => *n,
                None => 0,
            };

            self.tl_group_counts.insert(chapter.group.clone(), n + 1);

            if n + 1 > self.max_tl_count {
                self.max_tl = chapter.group.clone();
                self.max_tl_count = n + 1;
            }
        }

        // Append new chapters
        self.chapters.append(&mut chapters);
    }

    pub fn get_download_metadata(&self, language:&str, preferred_tl:&Option<String>, ranges:&Option<Vec<Range>>) -> Vec<&ChapterMetadata> {
        // Initially filter for language and range
        let chapters:HashMap<(&String, &String), Vec<&ChapterMetadata>> = self.chapters.iter()
            .filter(|m| m.language == language)
            .filter(|m| ranges.as_ref().map(|r| r.iter().any(|range| range.in_range(&m.volume, &m.chapter))).unwrap_or(true))
            .fold(HashMap::new(), |mut acc, m| {
                if let Some(ms) = acc.get_mut(&(&m.volume, &m.chapter)) {
                    ms.push(&m);
                } else {
                    acc.insert((&m.volume, &m.chapter), vec![&m]);
                }

                acc
            });

        // Filter for TL group
        chapters.iter()
            .map(|(_, ms)| match ms.len() { 
                1 => ms[0],
                _ => ms.iter() // Guaranteed != 0
                        .filter(|m| match preferred_tl {
                            Some(tl) => &m.group == tl,
                            None => m.group == self.max_tl,
                        })
                        .next()
                        .unwrap_or(&ms[0]),
            })
            .collect::<Vec<&ChapterMetadata>>()
    }
}

#[derive(Debug, Error)]
pub enum ChapterError {
    #[error("decoding error: {0}")]
    Decode(#[from] reqwest::Error),
    #[error("requester error: {0}")]
    Requester(#[from] RequesterError),
    #[error("image hash could not be found in its filename")]
    HashNotFound,
}

#[derive(Debug, Error)]
pub enum ImageDownloadError {
    #[error("requester error: {0}")]
    Requester(#[from] RequesterError),
    #[error("decoding error: {0}")]
    Decode(#[from] reqwest::Error),
    #[error("image has no content type")]
    NoContentType,
    #[error("illegible image mime type")]
    IllegibleMime(#[from] reqwest::header::ToStrError),
    #[error("unknown image mime type")]
    Mime,
    #[error("error saving image: {0}")]
    IO(#[from] std::io::Error),
    #[error("downloaded image has different hash to supplied one")]
    HashMismatch,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub id:String,
    pub volume: String,
    pub chapter: String,
    pub base_url: String,
    pub urls: Vec<Image>,
}
impl Chapter {
    pub async fn new(requester:&mut RateLimitedRequester, metadata:&ChapterMetadata) -> Result<Chapter, ChapterError> {
        let res = requester.request("cdn", &format!("/at-home/server/{}", metadata.id))
            .await?
            .json::<ChapterImageResponse>()
            .await?;

        let urls = res.chapter.data.iter()
            .map(|filename| Image::new(&res.chapter.hash, filename))
            .collect::<Result<Vec<Image>, ChapterError>>()?;

        Ok(Self {
            id: metadata.id.clone(),
            volume: metadata.volume.clone(),
            chapter: metadata.chapter.clone(),
            base_url: res.base_url,
            urls,
        })
    }

    pub fn get_volume(&self) -> String {
        match self.volume.parse::<f64>() {
            Ok(v) => format!("Volume {}", v),
            Err(_) => self.volume.clone(),
        }
    }

    pub fn get_chapter(&self) -> String {
        match self.chapter.parse::<f64>() {
            Ok(c) => format!("Chapter {}", c),
            Err(_) => self.chapter.clone(),
        }
    }

    fn assemble_folder_name(&self) -> String {
        let v = self.get_volume();
        let c = self.get_chapter();

        match (v.is_empty(), c.is_empty()) {
            (false, false) => format!("{}/{}", utils::escape_path(&v), utils::escape_path(&c)),
            (true, false) => utils::escape_path(&c),
            (false, true) => utils::escape_path(&v),
            (true, true) => String::from("Oneshot"),
        }
    }

    pub async fn download_to_folder(&self, requester:&mut RateLimitedRequester, master_directory:&Path, quiet:bool) -> Result<Option<ProgressBar<std::io::Stdout>>, ImageDownloadError> {
        let _ = requester.insert_source(&self.base_url, &self.base_url, Duration::from_millis(100)); // Ignore conflicting aliases
        let master_path = master_directory.join(Path::new(&self.assemble_folder_name()));
        fs::create_dir_all(&master_path)?;

        let mut pb = match quiet {
            false => Some(ProgressBar::new(self.urls.len() as u64)),
            true => None,
        };

        let digits = (self.urls.len() as f64).log10().floor() as usize + 1;

        for (i, image) in self.urls.iter().enumerate() {
            let res = requester.request(&self.base_url, image.url()).await?;

            // Derive extension from response Content-Type
            let content_type = res.headers().get("Content-Type")
                .ok_or(ImageDownloadError::NoContentType)?
                .to_str()?;

            let extension = mime_guess::get_mime_extensions_str(content_type)
                .ok_or(ImageDownloadError::Mime)?
                .iter().map(|s| *s)
                .next()
                .ok_or(ImageDownloadError::Mime)?;

            // Get the body
            let body = res.bytes().await?;

            // Verify the body. TODO: make this optional.
            if image.verify(&body) == false {
                return Err(ImageDownloadError::HashMismatch);
            }

            // I'm too lazy to do async file io
            let path = master_path.join(Path::new(&format!("{:0digits$}.{}", i + 1, extension, digits=digits)));
            let mut file = File::create(path)?;
            let _ = file.write_all(&body)?;

            if let Some(pb) = &mut pb {
                pb.inc();
            }
        }

        Ok(pb)
    }
}
