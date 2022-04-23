use crate::ratelimits::{ MasterRateLimits, MRL };

use thiserror::Error;
use regex::Regex;
use reqwest::{ self, Client, Response };

use std::time::{ Duration, SystemTimeError };

#[derive(Debug, Error)]
pub enum RequesterError {
    #[error("No host found in base url")]
    NoHost,
    #[error("reqwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("cannot access the system clock: {0}")]
    Time(#[from] SystemTimeError)
}

fn get_host(url:&str) -> Option<String> {
    lazy_static! {
        static ref HOST_RE:Regex = Regex::new(r"https?://([^/]+)/?.*").unwrap();
    }

    let m = HOST_RE.captures(url)?.get(1)?.as_str().to_string();
    Some(m)
}

pub struct Requester {
    base_url: String,
    host: String,
    client: Client,
    limits: MasterRateLimits,
}
impl Requester {
    pub fn new(base_url:&str, master:Duration) -> Result<Self, RequesterError> {
        Ok(Self {
            base_url: base_url.to_string(),
            host: get_host(base_url).ok_or(RequesterError::NoHost)?,
            client: Client::new(),
            limits: MasterRateLimits::from_master(master),
        })
    }

    pub async fn request(&mut self, path:&str) -> Result<Response, RequesterError> {
        let _ = self.limits.get_permission().await;
        let res = self.client.get(format!("{}{}", &self.base_url, path)).header("Host", &self.host).send().await?;
        self.limits.update_no_overwrite(res.headers());

        Ok(res)
    }
}
