use crate::ratelimits::{ RateLimiter, RateLimiterFunctions, ThreadedRateLimiter };

use async_recursion::async_recursion;
use chrono::{ TimeDelta, Utc };
use thiserror::Error;
use regex::Regex;
use reqwest::{ self, Client, Response, StatusCode };
use serde::de::DeserializeOwned;

use std::cmp;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::utils;

#[derive(Debug, Error)]
pub enum RequesterError {
    #[error("No host found in base url")]
    NoHost,
    #[error("Attempted to insert conflicting aliases")]
    ConflictingAlias,
    #[error("reqwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Error from the API: {0}")]
    APIError(String),
    #[error("Exceeded API rate limit")]
    RateLimited,
    #[error("API returned unexpected response: {0}")]
    UnexpectedResponse(String),
}

fn get_host(url:&str) -> Option<String> {
    lazy_static! {
        static ref HOST_RE:Regex = Regex::new(r"https?://([^/]+)/?.*").unwrap();
    }

    let m = HOST_RE.captures(url)?.get(1)?.as_str().to_string();
    Some(m)
}

pub struct RequesterSource {
    pub base_url: String,
    pub host: String,
    pub limiter: ThreadedRateLimiter,
}
impl RequesterSource {
    pub fn new(base_url:&str, timeout:Duration) -> Result<Self, RequesterError> {
        Ok(Self {
            base_url: base_url.to_string(),
            host: get_host(base_url).ok_or(RequesterError::NoHost)?,
            limiter: RateLimiter::new_threaded(timeout),
        })
    }
}

pub struct RateLimitedRequester {
    client: Client,
    sources: HashMap<String, RequesterSource>,
}
impl RateLimitedRequester {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            sources: HashMap::new(),
        }
    }

    pub fn new_with_defaults() -> Self {
        let mut requester = Self::new();
        // These three sources have been confirmed not to throw errors.
        let _ = requester.insert_source("main", "https://api.mangadex.org", Duration::from_millis(200)).unwrap();
        let _ = requester.insert_source("cdn", "https://api.mangadex.org", Duration::from_millis(1500)).unwrap();
        let _ = requester.insert_source("content", "https://uploads.mangadex.org", Duration::from_millis(200)).unwrap();

        requester
    }

    pub fn insert_source(&mut self, alias:&str, base_url:&str, timeout:Duration) -> Result<(), RequesterError> {
        if self.sources.contains_key(alias) {
            return Err(RequesterError::ConflictingAlias);
        }

        let source = RequesterSource::new(base_url, timeout)?;
        let _ = self.sources.insert(alias.to_string(), source);
        Ok(())
    }

    #[async_recursion]
    pub async fn request(&mut self, alias:&str, path:&str) -> Result<Response, RequesterError> {
        let mut source = self.sources.get_mut(alias);
        if let Some(ref mut s) = source {
            let _ = s.limiter.get_permission().await;
            s.limiter.update();
        }

        let base_url = match source {
            Some(ref s) => &s.base_url,
            None => "",
        };

        let mut req = self.client.get(format!("{}{}", base_url, path)).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:123.0) Gecko/20100101 Firefox/123.0");
        if let Some(s) = source {
            req = req.header("Host", &s.host);
        }

        let res = req.send().await?;
        if !res.status().is_success() {
            // Check is we're being rate-limited
            if res.status() == StatusCode::TOO_MANY_REQUESTS {
                let retry = utils::get_retry_after(&res).ok_or(RequesterError::RateLimited)?;
                let delay = cmp::max(TimeDelta::new(0, 0).unwrap(), retry - Utc::now()).to_std().map_err(|_| RequesterError::RateLimited)?;
                thread::sleep(delay);
                return self.request(alias, path).await;
            } else {
            let msg = res.text().await?;
                return Err(RequesterError::APIError(msg));
            }
        }

        Ok(res)
    }

    pub async fn request_json<T:DeserializeOwned>(&mut self, alias:&str, path:&str) -> Result<T, RequesterError> {
        let body = self.request(alias, path).await?.text().await?;
        serde_json::from_str(&body)
            .map_err(|_| RequesterError::UnexpectedResponse(body))
    }
}
