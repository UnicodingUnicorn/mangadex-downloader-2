#[macro_use]
extern crate lazy_static;

mod api;
mod chapter;
mod manga;
mod ratelimits;
mod requester;
mod types;
mod utils;

use api::API;
use chapter::ChapterMetadata;

#[tokio::main]
async fn main() {
    let mut api = API::new();
    let manga_metadata = api.get_manga_metadata("https://mangadex.org/title/348966d0-c807-45cf-9260-8adf006a9da6/kono-bijutsubu-ni-wa-mondai-ga-aru").await.unwrap();
    let chapter_metadata = api.get_chapter_metadata(&manga_metadata).await.unwrap();
    let en_chapter_metadata = chapter_metadata.iter()
        .filter(|m| m.language == "en")
        .map(|m| m.clone())
        .collect::<Vec<ChapterMetadata>>();
    let chapters = api.get_chapters(&en_chapter_metadata).await.unwrap();
    println!("{:#?}", chapters);
    api.download_chapters(&chapters, "output").await.unwrap();
}
