use crate::requester::{ RateLimitedRequester, RequesterError };
use crate::types::{ ChapterData, ChapterDataResponse, ChapterImageResponse, MangaDataResponse };

use regex::Regex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChapterError {
    #[error("decoding error: {0}")]
    Decode(#[from] reqwest::Error),
    #[error("requester error: {0}")]
    Requester(#[from] RequesterError),
}

#[derive(Debug)]
pub struct Chapter {
    volume: String,
    chapter: String,
    language: String,
    urls: Vec<String>
}
impl Chapter {
    pub async fn new(requester:&mut RateLimitedRequester, raw:&ChapterData) -> Result<Self, ChapterError> {
        let res = requester.request("cdn", &format!("/at-home/server/{}", raw.id))
            .await?;

        let res = res.json::<ChapterImageResponse>()
            .await?;

        let urls = res.chapter.data.iter()
            .map(|datum| format!("{}/data/{}/{}", res.base_url, res.chapter.hash, datum))
            .collect::<Vec<String>>();

        Ok(Self {
            volume: raw.attributes.volume.clone(),
            chapter: raw.attributes.chapter.clone(),
            language: raw.attributes.language.clone(),
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
}

fn get_id(url:&str) -> Option<String> {
    lazy_static! {
        static ref ID_RE:Regex = Regex::new(r"https?://mangadex\.org/title/((?:[0-9a-fA-F]+-?)+)/?.*").unwrap();
    }

    let id = ID_RE.captures(url)?.get(1)?.as_str().to_string();
    Some(id)
}

#[derive(Debug)]
pub struct Manga {
    pub title: String,
    pub available_languages: Vec<String>,
    pub chapters: Vec<Chapter>,
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

        let chapters = Chapter::get_all(&mut requester, &id, preferred_language).await?;

        Ok(Self{
            title,
            available_languages: raw_manga_data.data.attributes.available_languages,
            chapters,
        })
    }
}
