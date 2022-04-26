#[macro_use]
extern crate lazy_static;

mod manga;
mod ratelimits;
mod requester;
mod types;

use requester::RateLimitedRequester;
use types::ChapterDataResponse;

use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut requester = RateLimitedRequester::new("https://api.mangadex.org", Duration::from_secs(60)).unwrap();
    let manga = manga::Manga::new(&mut requester, "https://mangadex.org/title/348966d0-c807-45cf-9260-8adf006a9da6/kono-bijutsubu-ni-wa-mondai-ga-aru", "en").await.unwrap();
    // let res = requester.request("/manga/348966d0-c807-45cf-9260-8adf006a9da6/feed").await.unwrap().json::<ChapterDataResponse>().await.unwrap();
    println!("{:#?}", manga);
}
