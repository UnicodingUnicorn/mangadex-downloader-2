use crate::types::MangaDataResponse;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MangaMetadata {
    pub id: String,
    pub titles: HashMap<String, String>,
    pub alt_titles: HashMap<String, Vec<String>>,
    pub languages: Vec<String>,
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

        Self {
            id,
            titles: raw.data.attributes.title,
            alt_titles,
            languages: raw.data.attributes.available_languages,
        }
    }

    pub fn get_title(&self, preferred_language:&str) -> Option<String> {
        match self.titles.get(preferred_language) {
            Some(title) => Some(title.to_string()),
            None => self.titles.iter().next().map(|(_, title)| title.to_string()),
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

        println!("Available Languages: {}", self.languages.join(", "));
    }
}
