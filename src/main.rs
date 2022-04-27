#[macro_use]
extern crate lazy_static;

mod manga;
mod ratelimits;
mod requester;
mod types;

#[tokio::main]
async fn main() {
    let manga = manga::Manga::new("https://mangadex.org/title/348966d0-c807-45cf-9260-8adf006a9da6/kono-bijutsubu-ni-wa-mondai-ga-aru", "en").await.unwrap();
    println!("{:#?}", manga);
}
