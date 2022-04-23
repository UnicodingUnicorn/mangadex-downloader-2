use reqwest::header::HeaderMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{ Context, Poll };
use std::sync::{ Arc, RwLock };
use std::time::{ Duration, Instant, SystemTime, SystemTimeError, UNIX_EPOCH };
use std::thread;

pub struct RateLimits {
    current: u64,
    rollover: Duration,
}
impl RateLimits {
    pub fn from_headers(headers:&HeaderMap) -> Option<Self> {
        let current = u64::from_str_radix(headers.get("X-RateLimit-Remaining")?.to_str().ok()?, 10).ok()?;
        let raw_rollover = u64::from_str_radix(headers.get("X-RateLimit-Retry-After")?.to_str().ok()?, 10).ok()?;
        let rollover = Duration::from_secs(raw_rollover);

        Some(Self {
            current,
            rollover,
        })
    }

    pub fn can_query(&self) -> Result<bool, SystemTimeError> {
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?;
        Ok(current_time > self.rollover || self.current > 0)
    }

    pub fn get_timeout(&self) -> Result<Duration, SystemTimeError> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
        Ok(self.rollover.saturating_sub(now))
    }
}

pub trait MRL {
    fn from_master(master:Duration) -> Self;
    fn from_headers(master:Duration, headers:&HeaderMap) -> Self;
    fn can_query(&self) -> Result<bool, SystemTimeError>;
    fn get_timeout(&self) -> Result<Duration, SystemTimeError>;
    fn update(&mut self, headers:&HeaderMap);
    fn update_no_overwrite(&mut self, headers:&HeaderMap);
    fn get_permission(&self) -> RateLimitFuture;
}
pub type MasterRateLimits = Arc<RwLock<InnerMasterRateLimits>>;
pub struct InnerMasterRateLimits {
    last_hit: Instant,
    master: Duration,
    rate_limit: Option<RateLimits>,
}
impl MRL for MasterRateLimits {
    fn from_master(master:Duration) -> Self {
        let rl = InnerMasterRateLimits {
            last_hit: Instant::now() - master,
            master,
            rate_limit: None,
        };

        Arc::new(RwLock::new(rl))
    }

    fn from_headers(master:Duration, headers:&HeaderMap) -> Self {
        let rl = InnerMasterRateLimits {
            last_hit: Instant::now(),
            master,
            rate_limit: RateLimits::from_headers(headers),
        };

        Arc::new(RwLock::new(rl))
    }

    fn can_query(&self) -> Result<bool, SystemTimeError> {
        let mrl = self.read().unwrap();
        match &mrl.rate_limit {
            Some(rl) => rl.can_query(),
            None => Ok(Instant::now().duration_since(mrl.last_hit) > mrl.master),
        }
    }

    fn get_timeout(&self) -> Result<Duration, SystemTimeError> {
        let mrl = self.read().unwrap();
        match &mrl.rate_limit {
            Some(rl) => rl.get_timeout(),
            None => Ok(match (mrl.last_hit + mrl.master).checked_duration_since(Instant::now()) {
                Some(d) => d,
                None => Duration::ZERO,
            }),
        }
    }

    fn update(&mut self, headers:&HeaderMap) {
        let mut mrl = self.write().unwrap();
        mrl.last_hit = Instant::now();
        mrl.rate_limit = RateLimits::from_headers(headers);
    }

    fn update_no_overwrite(&mut self, headers:&HeaderMap) {
        let mut mrl = self.write().unwrap();
        mrl.last_hit = Instant::now();
        if let Some(rl) = RateLimits::from_headers(headers) {
            mrl.rate_limit = Some(rl);
        }
    }

    fn get_permission(&self) -> RateLimitFuture {
        RateLimitFuture {
            rl: self.clone(),
        }
    }
}

pub struct RateLimitFuture {
    rl: MasterRateLimits,
}
impl Future for RateLimitFuture {
    type Output = Result<(), SystemTimeError>;
    fn poll(self:Pin<&mut Self>, ctx:&mut Context) -> Poll<Self::Output> {
        match self.rl.can_query() {
            Ok(true) => Poll::Ready(Ok(())),
            Ok(false) => match self.rl.get_timeout() {
                Ok(timeout) => {
                    let waker = ctx.waker().clone();
                    thread::spawn(move || {
                        thread::sleep(timeout);
                        waker.wake();
                    });

                    Poll::Pending
                },
                Err(e) => Poll::Ready(Err(e)),
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
