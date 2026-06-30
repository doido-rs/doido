pub mod config;
pub mod environment;
pub mod global;
pub mod memory;
pub mod namespaced;
pub mod registry;
pub mod store;

#[cfg(feature = "cache-redis")]
pub mod redis_store;
#[cfg(feature = "cache-memcache")]
pub mod memcache_store;

pub use config::{CacheBackend, CacheConfig};
pub use environment::Environment;
pub use global::init as init_cache;
pub use memory::MemoryStore;
pub use namespaced::NamespacedStore;
pub use registry::CacheRegistry;
pub use store::CacheStore;

#[cfg(feature = "cache-redis")]
pub use redis_store::RedisStore;
#[cfg(feature = "cache-memcache")]
pub use memcache_store::MemcacheStore;
