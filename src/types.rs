use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct TagDataAttributes {
    pub name: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagData {
    pub attributes: TagDataAttributes,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MangaDataAttributes {
    pub title: HashMap<String, String>,
    #[serde(rename="altTitles")]
    pub alt_titles: Vec<HashMap<String, String>>,
    #[serde(rename="availableTranslatedLanguages")]
    pub available_languages: Vec<Option<String>>,
    pub description: HashMap<String, String>,
    pub tags: Vec<TagData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MangaData {
    pub attributes: MangaDataAttributes,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MangaDataResponse {
    pub data: MangaData,
}

// We only care about 'scanlation_group' attributes
#[derive(Debug, Deserialize, Serialize)]
pub struct RawChapterRelationshipAttributes {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawChapterRelationship {
    #[serde(rename="type")]
    pub id: String,
    pub attributes: Option<RawChapterRelationshipAttributes>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterAttributes {
    pub volume: Option<String>,
    pub chapter: Option<String>,
    #[serde(rename="translatedLanguage")]
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterData {
    pub id: String,
    pub attributes: ChapterAttributes,
    pub relationships: Vec<RawChapterRelationship>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterDataResponse {
    pub data: Vec<ChapterData>,
    pub limit: u64,
    pub offset: u64,
    pub total: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterImageData {
    pub hash: String,
    pub data: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChapterImageResponse {
    #[serde(rename="baseUrl")]
    pub base_url: String,
    pub chapter: ChapterImageData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CoverArtAttributes {
    pub volume: Option<String>,
    #[serde(rename="fileName")]
    pub file_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CoverArtData {
    pub attributes: CoverArtAttributes,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CoverArtResponse {
    pub data: Vec<CoverArtData>,
    pub limit: u64,
    pub offset: u64,
    pub total: u64,
}
