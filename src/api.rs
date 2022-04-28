use crate::chapter::{ Chapter, ChapterError, ChapterDownloadError, ChapterMetadata };
use crate::manga::MangaMetadata;
use crate::requester::{ RateLimitedRequester, RequesterError };
use crate::types::{ ChapterDataResponse, MangaDataResponse };
use crate::utils;

use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum APIError {
    #[error("error making request: {0}")]
    Requester(#[from] RequesterError),
    #[error("error decoding response body: {0}")]
    Decoding(#[from] reqwest::Error),
    #[error("manga id could not be parsed from the given url")]
    NoID,
    #[error("error retrieving chapter information: {0}")]
    Chapter(#[from] ChapterError),
    #[error("error downloading chapter images: {0}")]
    ChapterDownload(#[from] ChapterDownloadError),
}

pub struct API {
    requester: RateLimitedRequester,
}
impl API {
    pub fn new() -> Self {
        Self {
            requester: RateLimitedRequester::new_with_defaults(),
        }
    }

    pub async fn get_manga_metadata(&mut self, url:&str) -> Result<MangaMetadata, APIError> {
        let id = utils::get_id(url).ok_or(APIError::NoID)?;
        let raw_manga_data = self.requester.request("main", &format!("/manga/{}", id))
            .await?
            .json::<MangaDataResponse>()
            .await?;

        Ok(MangaMetadata::from_response(id, raw_manga_data))
    }

    pub async fn get_chapter_metadata(&mut self, manga_metadata:&MangaMetadata) -> Result<Vec<ChapterMetadata>, APIError> {
        let mut chapters = vec![];
        let mut i = 0;
        loop {
            let res = self.requester.request("main", &format!("/manga/{}/feed?offset={}", &manga_metadata.id, i))
                .await?
                .json::<ChapterDataResponse>()
                .await?;

            let mut new_chapters = ChapterMetadata::from_response(res.data);
            chapters.append(&mut new_chapters);

            i += res.limit;
            if i > res.total {
                break;
            }
        }

        Ok(chapters)
    }

    pub async fn get_chapters(&mut self, chapter_metadata:&[ChapterMetadata]) -> Result<Vec<Chapter>, APIError> {
        let mut chapters = vec![];
        let mut iter = chapter_metadata.iter();
        while let Some(metadata) = iter.next() {
            let chapter = Chapter::new(&mut self.requester, &metadata).await?;
            chapters.push(chapter);
        }

        Ok(chapters)
    }

    pub async fn download_chapters(&mut self, chapters:&[Chapter], master_directory:&Path) -> Result<(), APIError> {
        let mut iter = chapters.iter();
        while let Some(chapter) = iter.next() {
            chapter.download_to_folder(&mut self.requester, master_directory).await?;
        }

        Ok(())
    }
}
