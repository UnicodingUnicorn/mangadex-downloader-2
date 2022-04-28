use crate::requester::{ RateLimitedRequester, RequesterError };
use crate::types::{ ChapterData, ChapterDataResponse, ChapterImageResponse, MangaDataResponse };

use std::path::Path;
use std::fs::{ self, File };
use std::io::Write;
use std::time::Duration;

use regex::Regex;
use thiserror::Error;

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

#[derive(Debug)]
pub struct Chapter {
    volume: String,
    chapter: String,
    language: String,
    base_url: String,
    urls: Vec<String>
}
impl Chapter {
    pub async fn new(requester:&mut RateLimitedRequester, raw:&ChapterData) -> Result<Self, ChapterError> {
        let res = requester.request("cdn", &format!("/at-home/server/{}", raw.id))
            .await?;

        let res = res.json::<ChapterImageResponse>()
            .await?;

        let urls = res.chapter.data.iter()
            .map(|datum| format!("/data/{}/{}", res.chapter.hash, datum))
            .collect::<Vec<String>>();

        Ok(Self {
            volume: raw.attributes.volume.clone(),
            chapter: raw.attributes.chapter.clone(),
            language: raw.attributes.language.clone(),
            base_url: res.base_url,
            urls,
        })
    }

    pub async fn get_page(requester:&mut RateLimitedRequester, id:&str, n:u64, language:&str) -> Result<(Vec<Self>, u64, bool), ChapterError> {
        let res = requester.request("main", &format!("/manga/{}/feed?offset={}", id, n))
            .await?
            .json::<ChapterDataResponse>()
            .await?;

        let mut iter = res.data.iter().filter(|datum| datum.attributes.language == language);
        let mut data = vec![];
        while let Some(datum) = iter.next() {
            let c = Self::new(requester, &datum).await?;
            data.push(c);
        }

        Ok((data, res.limit, res.limit + res.offset < res.total))
    }

    pub async fn get_all(requester:&mut RateLimitedRequester, id:&str, language:&str) -> Result<Vec<Self>, ChapterError> {
        let mut chapters = vec![];
        let mut i = 0;
        let mut c = true;
        while c {
            let (mut data, n, cont) = Self::get_page(requester, id, i, language).await?;
            chapters.append(&mut data);

            i += n;
            c = cont;
        }

        Ok(chapters)
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

    pub async fn download_to_folder(&self, requester:&mut RateLimitedRequester, master_directory:&str) -> Result<(), ChapterDownloadError> {
        let _ = requester.insert_source(&self.base_url, &self.base_url, Duration::from_millis(100)); // Ignore conflicting aliases
        let master_path = Path::new(master_directory).join(Path::new(&format!("{}/{}", self.get_volume(), self.get_chapter())));
        fs::create_dir_all(&master_path)?;

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
        }

        Ok(())
    }
}


#[derive(Debug, Error)]
pub enum MangaError {
    #[error("id could not be parsed from the given url")]
    NoID,
    #[error("no title could be retrieved")]
    NoTitle,
    #[error("decoding error: {0}")]
    Decode(#[from] reqwest::Error),
    #[error("requester error: {0}")]
    Requester(#[from] RequesterError),
    #[error("error retrieving chapter data: {0}")]
    Chapter(#[from] ChapterError),
    #[error("error downloading chapter: {0}")]
    ChapterDownload(#[from] ChapterDownloadError),
}

fn get_id(url:&str) -> Option<String> {
    lazy_static! {
        static ref ID_RE:Regex = Regex::new(r"https?://mangadex\.org/title/((?:[0-9a-fA-F]+-?)+)/?.*").unwrap();
    }

    let id = ID_RE.captures(url)?.get(1)?.as_str().to_string();
    Some(id)
}

pub struct Manga {
    pub id: String,
    pub title: String,
    pub available_languages: Vec<String>,
    pub chapters: Option<Vec<Chapter>>,
    requester: RateLimitedRequester,
}
impl Manga {
    pub async fn new(url:&str, preferred_language:&str) -> Result<Self, MangaError> {
        let id = get_id(url).ok_or(MangaError::NoID)?;
        let mut requester = RateLimitedRequester::new_with_defaults()?;
        let raw_manga_data = requester.request("main", &format!("/manga/{}", id))
            .await?
            .json::<MangaDataResponse>()
            .await?;

        let title = match raw_manga_data.data.attributes.title.get(preferred_language) {
            Some(title) => title.clone(),
            None => match raw_manga_data.data.attributes.title.iter().next() {
                Some((_, title)) => title.clone(),
                None => return Err(MangaError::NoTitle),
            },
        };

        Ok(Self{
            id,
            title,
            available_languages: raw_manga_data.data.attributes.available_languages,
            chapters: None,
            requester,
        })
    }

    pub async fn populate_chapters(&mut self, preferred_language:&str) -> Result<(), MangaError> {
        let chapters = Chapter::get_all(&mut self.requester, &self.id, preferred_language).await?;
        self.chapters = Some(chapters);

        Ok(())
    }

    pub async fn download_chapters(&mut self) -> Result<(), MangaError> {
        if let Some(chapter) = self.chapters.as_ref().unwrap().iter().next() {
            let _ = chapter.download_to_folder(&mut self.requester, "").await?;
        }

        Ok(())
    }
}
