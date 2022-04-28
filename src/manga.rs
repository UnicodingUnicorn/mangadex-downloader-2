use crate::chapter::{ Chapter, ChapterError, ChapterDownloadError };
use crate::requester::{ RateLimitedRequester, RequesterError };
use crate::types::MangaDataResponse;

use regex::Regex;
use thiserror::Error;

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
