use crate::types::MangaDataResponse;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MangaMetadata {
    pub id: String,
    pub titles: HashMap<String, String>,
    pub alt_titles: HashMap<String, Vec<String>>,
    pub languages: Vec<String>,
    pub descriptions: HashMap<String, String>,
    pub tags: Vec<HashMap<String, String>>,
}
impl MangaMetadata {
    pub fn from_response(id:String, raw:MangaDataResponse) -> Self {
        let alt_titles = raw.data.attributes.alt_titles.iter()
            .map(|at| at.iter())
            .flatten()
            .fold(HashMap::new(), |mut acc:HashMap<String, Vec<String>>, (lang, title)| {
                if let Some(ats) = acc.get_mut(lang) {
                    ats.push(title.to_string());
                } else {
                    acc.insert(lang.to_string(), vec![title.to_string()]);
                }

                acc
            });

        let languages = raw.data.attributes.available_languages.iter()
            .filter(|al| al.is_some())
            .map(|al| al.as_ref().unwrap().to_string()) // Guaranteed Some
            .collect::<Vec<String>>();

        let tags = raw.data.attributes.tags.iter()
            .map(|t| t.attributes.name.clone())
            .collect::<Vec<HashMap<String, String>>>();

        Self {
            id,
            titles: raw.data.attributes.title,
            alt_titles,
            languages,
            descriptions: raw.data.attributes.description,
            tags,
        }
    }

    pub fn get_title(&self, preferred_language:&str) -> Option<String> {
        match self.titles.get(preferred_language) {
            Some(title) => Some(title.to_string()),
            None => self.titles.iter().next().map(|(_, title)| title.to_string()),
        }
    }

    pub fn get_description(&self, preferred_language:&str) -> Option<String> {
        match self.descriptions.get(preferred_language) {
            Some(description) => Some(description.to_string()),
            None => self.descriptions.iter().next().map(|(_, description)| description.to_string()),
        }
    }

    pub fn print(&self) {
        println!("ID: {}", self.id);

        let titles = self.titles.iter()
            .map(|(lang, title)| format!("\t- {} ({})", title, lang))
            .intersperse("\n".to_string())
            .collect::<String>();

        println!("Titles:",);
        println!("{}", titles);

        let alt_titles = self.alt_titles.iter()
            .map(|(lang, titles)| titles.iter().map(|title| format!("\t- {} ({})", title, lang.clone())))
            .flatten()
            .intersperse("\n".to_string())
            .collect::<String>();

        println!("Alternative Titles:");
        println!("{}", alt_titles);

        let tags = self.tags.iter()
            .map(|t| t.iter().map(|(_, v)| v))
            .flatten()
            .map(|t| t.to_string())
            .intersperse(", ".to_string())
            .collect::<String>();

        println!("Tags: {}", tags);

        println!("Available Languages: {}", self.languages.join(", "));

        println!("-");
        println!("Descriptions:");
        for (language, description) in self.descriptions.iter() {
            println!("({}) {}", language, description);
            println!("");
        }
    }
}
