use doido_cache::MemoryStore;
use doido_controller::rate_limit::RateLimiter;
use std::sync::Arc;

#[tokio::test]
async fn allows_up_to_the_limit_then_denies() {
    let limiter = RateLimiter::new(Arc::new(MemoryStore::new()), 2, 60);
    assert!(limiter.check("ip-1").await, "1st request allowed");
    assert!(limiter.check("ip-1").await, "2nd request allowed");
    assert!(!limiter.check("ip-1").await, "3rd request over the limit");
}

#[tokio::test]
async fn limits_are_per_key() {
    let limiter = RateLimiter::new(Arc::new(MemoryStore::new()), 1, 60);
    assert!(limiter.check("ip-a").await);
    assert!(!limiter.check("ip-a").await, "ip-a exhausted");
    assert!(limiter.check("ip-b").await, "ip-b has its own budget");
}
