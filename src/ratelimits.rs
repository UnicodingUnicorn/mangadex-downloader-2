use std::future::Future;
use std::pin::Pin;
use std::task::{ Context, Poll };
use std::sync::{ Arc, RwLock };
use std::time::{ Duration, Instant };
use std::thread;

// Simple timeout based rate limiter
#[derive(Debug)]
pub struct RateLimiter {
    last_hit: Instant,
    timeout: Duration,
}
impl RateLimiter {
    pub fn new(timeout:Duration) -> Self {
        Self {
            last_hit: Instant::now() - timeout,
            timeout,
        }
    }

    pub fn new_threaded(timeout:Duration) -> ThreadedRateLimiter {
        Arc::new(RwLock::new(Self::new(timeout)))
    }

    pub fn can_query(&self) -> bool {
        Instant::now() - self.last_hit > self.timeout
    }

    pub fn get_timeout(&self) -> Duration {
        self.last_hit.checked_add(self.timeout)
            .unwrap_or(Instant::now())
            .saturating_duration_since(Instant::now())
    }

    pub fn update(&mut self) {
        self.last_hit = Instant::now();
    }
}

pub trait RateLimiterFunctions {
    fn can_query(&self) -> bool;
    fn get_timeout(&self) -> Duration;
    fn update(&mut self);
    fn get_permission(&self) -> RateLimiterFuture;
}

pub type ThreadedRateLimiter = Arc<RwLock<RateLimiter>>;
impl RateLimiterFunctions for ThreadedRateLimiter {
    fn can_query(&self) -> bool {
        let rl = self.read().unwrap();
        rl.can_query()
    }

    fn get_timeout(&self) -> Duration {
        let rl = self.read().unwrap();
        rl.get_timeout()
    }

    fn update(&mut self) {
        let mut rl = self.write().unwrap();
        rl.update();
    }

    fn get_permission(&self) -> RateLimiterFuture {
        RateLimiterFuture {
            rl: self.clone(),
        }
    }
}

pub struct RateLimiterFuture {
    rl: ThreadedRateLimiter,
}
impl Future for RateLimiterFuture {
    type Output = ();
    fn poll(self:Pin<&mut Self>, ctx:&mut Context) -> Poll<Self::Output> {
        if self.rl.can_query() {
            Poll::Ready(())
        } else {
            let timeout = self.rl.get_timeout();
            let waker = ctx.waker().clone();
            thread::spawn(move || {
                thread::sleep(timeout);
                waker.wake();
            });

            Poll::Pending
        }
    }
}
