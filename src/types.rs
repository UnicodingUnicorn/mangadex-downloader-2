use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct MangaDataAttributes {
    pub title: HashMap<String, String>,
    #[serde(rename="altTitles")]
    pub alt_titles: Vec<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MangaData {
    pub attributes: MangaDataAttributes,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MangaDataResponse {
    pub data: MangaData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterAttributes {
    pub volume: String,
    pub chapter: String,
    #[serde(rename="translatedLanguage")]
    pub language: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterData {
    pub id: String,
    pub attributes: ChapterAttributes,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterDataResponse {
    pub data: Vec<ChapterData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterImageData {
    pub data: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterImageResponse {
    #[serde(rename="baseUrl")]
    pub base_url: String,
    pub chapter: ChapterImageData,
}
