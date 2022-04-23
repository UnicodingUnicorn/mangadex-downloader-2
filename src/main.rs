#[macro_use]
extern crate lazy_static;

mod ratelimits;
mod requester;

use ratelimits::RateLimits;
use requester::Requester;

use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut requester = Requester::new("https://api.mangadex.org", Duration::from_secs(60)).unwrap();

    let res = requester.request("/at-home/server/bd359419-4dc3-4020-9ff1-e4d7e6f75aa5").await.unwrap();
    // println!("{}", res.status());
    // println!("{}", res.text().await.unwrap());
    let rl = RateLimits::from_headers(res.headers()).unwrap();
    println!("{:?}", rl.get_timeout());
}
