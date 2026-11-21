pub mod codec;
pub mod config;
pub mod fetch;
pub mod global;
pub mod memory;
pub mod multi;
pub mod namespaced;
pub mod registry;
pub mod store;
pub mod versioning;

#[cfg(feature = "cache-memcache")]
pub mod memcache_store;
#[cfg(feature = "cache-redis")]
pub mod redis_store;

pub use config::{CacheBackend, CacheConfig, MultiCacheConfig};
pub use global::default_initializer;
pub use global::init as init_cache;
#[doc(hidden)]
pub use global::reset_for_test;
pub use global::{set_store, store, try_store};
pub use memory::MemoryStore;
pub use namespaced::NamespacedStore;
pub use registry::CacheRegistry;
pub use store::CacheStore;

#[cfg(feature = "cache-memcache")]
pub use memcache_store::MemcacheStore;
#[cfg(feature = "cache-redis")]
pub use redis_store::RedisStore;
