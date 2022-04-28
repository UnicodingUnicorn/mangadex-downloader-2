use crate::requester::{ RateLimitedRequester, RequesterError };
use crate::types::{ ChapterData, ChapterImageResponse };

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
}
impl ChapterMetadata {
    pub fn from_chapter_data(raw:ChapterData) -> Self {
        Self {
            id: raw.id,
            volume: raw.attributes.volume,
            chapter: raw.attributes.chapter,
            language: raw.attributes.language,
        }
    }

    pub fn from_response(mut raw:Vec<ChapterData>) -> Vec<Self> {
        raw.drain(..).map(|r| Self::from_chapter_data(r)).collect::<Vec<Self>>()
    }
}

#[derive(Debug, Error)]
pub enum ChapterError {
    #[error("decoding error: {0}")]
    Decode(#[from] reqwest::Error),
    #[error("requester error: {0}")]
    Requester(#[from] RequesterError),
}

#[derive(Debug, Error)]
pub enum ChapterDownloadError {
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
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub id:String,
    pub volume: String,
    pub chapter: String,
    pub base_url: String,
    pub urls: Vec<String>,
}
impl Chapter {
    pub async fn new(requester:&mut RateLimitedRequester, metadata:&ChapterMetadata) -> Result<Chapter, ChapterError> {
        let res = requester.request("cdn", &format!("/at-home/server/{}", metadata.id))
            .await?
            .json::<ChapterImageResponse>()
            .await?;

        let urls = res.chapter.data.iter()
           .map(|datum| format!("/data/{}/{}", res.chapter.hash, datum))
           .collect::<Vec<String>>();

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

    pub async fn download_to_folder(&self, requester:&mut RateLimitedRequester, master_directory:&Path) -> Result<(), ChapterDownloadError> {
        let _ = requester.insert_source(&self.base_url, &self.base_url, Duration::from_millis(100)); // Ignore conflicting aliases
        let master_path = master_directory.join(Path::new(&format!("{}/{}", self.get_volume(), self.get_chapter())));
        fs::create_dir_all(&master_path)?;

        let mut pb = ProgressBar::new(self.urls.len() as u64);
        for (i, url) in self.urls.iter().enumerate() {
            let res = requester.request(&self.base_url, &url).await?;
            let content_type = res.headers().get("Content-Type")
                .ok_or(ChapterDownloadError::NoContentType)?
                .to_str()?;

            let extension = mime_guess::get_mime_extensions_str(content_type)
                .ok_or(ChapterDownloadError::Mime)?
                .iter().map(|s| *s)
                .next()
                .ok_or(ChapterDownloadError::Mime)?;

            let body = res.bytes().await?;

            // I'm too lazy to do async file io
            let path = master_path.join(Path::new(&format!("{}.{}", i + 1, extension)));
            let mut file = File::create(path)?;
            let _ = file.write_all(&body)?;

            pb.inc();
        }

        pb.finish();
        Ok(())
    }
}
