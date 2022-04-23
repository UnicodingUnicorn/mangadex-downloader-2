#[macro_use]
extern crate lazy_static;

mod ratelimits;
mod requester;
mod types;

use ratelimits::RateLimits;
use requester::Requester;
use types::ChapterImageResponse;

use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut requester = Requester::new("https://api.mangadex.org", Duration::from_secs(60)).unwrap();
    let res = requester.request("/at-home/server/3a203b80-b102-4454-a5ea-5f55e32259e5").await.unwrap().json::<ChapterImageResponse>().await.unwrap();
    println!("{:#?}", res);
}
