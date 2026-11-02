//! Secure passwords and tokens (Rails `has_secure_password` / `has_secure_token`).
//!
//! New passwords are hashed with **Argon2id** (OWASP minimum parameters by default).
//! Legacy **bcrypt** digests (`$2…`) remain verifiable; [`needs_rehash`] marks them
//! for transparent upgrade on the next successful login.

use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use bcrypt::{hash as bcrypt_hash, verify as bcrypt_verify};
use doido_core::Result;
use password_hash::{rand_core::OsRng, SaltString};
use std::sync::{Mutex, OnceLock, RwLock};

/// OWASP minimum Argon2id: 19 MiB, 2 iterations, parallelism 1.
pub const DEFAULT_MEMORY_KIB: u32 = 19_456;
pub const DEFAULT_ITERATIONS: u32 = 2;
pub const DEFAULT_PARALLELISM: u32 = 1;

/// Default maximum password length (Unicode scalar count).
pub const DEFAULT_MAX_PASSWORD_LENGTH: usize = 128;

/// Argon2id cost parameters (tunable via [`configure_password_hashing`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordHashConfig {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Default for PasswordHashConfig {
    fn default() -> Self {
        Self {
            memory_kib: DEFAULT_MEMORY_KIB,
            iterations: DEFAULT_ITERATIONS,
            parallelism: DEFAULT_PARALLELISM,
        }
    }
}

impl PasswordHashConfig {
    /// Low-cost parameters for unit tests (do not use in production).
    pub fn fast_test() -> Self {
        Self {
            memory_kib: 8_192,
            iterations: 1,
            parallelism: 1,
        }
    }
}

struct PasswordRuntime {
    hash_config: PasswordHashConfig,
    max_length: usize,
    pepper: Option<Vec<u8>>,
}

static RUNTIME: OnceLock<RwLock<PasswordRuntime>> = OnceLock::new();

/// Serialises configure/hash/verify so parallel tests do not clobber pepper/params.
static PASSWORD_SERIAL: Mutex<()> = Mutex::new(());

fn with_password_serial<R>(f: impl FnOnce() -> R) -> R {
    let _guard = PASSWORD_SERIAL
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f()
}

fn runtime() -> &'static RwLock<PasswordRuntime> {
    RUNTIME.get_or_init(|| {
        RwLock::new(PasswordRuntime {
            hash_config: PasswordHashConfig::default(),
            max_length: DEFAULT_MAX_PASSWORD_LENGTH,
            pepper: None,
        })
    })
}

/// Install Argon2 parameters, optional pepper, and max password length (boot / tests).
pub fn configure_password_hashing(
    hash_config: PasswordHashConfig,
    pepper: Option<Vec<u8>>,
    max_password_length: usize,
) {
    with_password_serial(|| {
        let mut guard = runtime().write().expect("password runtime lock");
        guard.hash_config = hash_config;
        guard.pepper = pepper;
        guard.max_length = max_password_length.max(1);
    });
}

fn active_config() -> PasswordHashConfig {
    runtime().read().expect("password runtime lock").hash_config
}

fn active_max_length() -> usize {
    runtime().read().expect("password runtime lock").max_length
}

fn active_pepper() -> Option<Vec<u8>> {
    runtime()
        .read()
        .expect("password runtime lock")
        .pepper
        .clone()
}

fn peppered_secret(password: &str) -> Vec<u8> {
    if let Some(pepper) = active_pepper() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac =
            HmacSha256::new_from_slice(&pepper).expect("pepper length valid for HMAC-SHA256");
        mac.update(password.as_bytes());
        mac.finalize().into_bytes().to_vec()
    } else {
        password.as_bytes().to_vec()
    }
}

/// Return `true` when `password` exceeds the configured maximum length.
pub fn password_too_long(password: &str) -> bool {
    with_password_serial(|| password.chars().count() > active_max_length())
}

/// Hash a password with the active Argon2id configuration.
pub fn hash_password(password: &str) -> Result<String> {
    with_password_serial(|| hash_password_with_config_inner(password, active_config()))
}

/// Hash a password with explicit Argon2id parameters.
pub fn hash_password_with_config(password: &str, config: PasswordHashConfig) -> Result<String> {
    with_password_serial(|| hash_password_with_config_inner(password, config))
}

fn hash_password_with_config_inner(password: &str, config: PasswordHashConfig) -> Result<String> {
    if password.chars().count() > active_max_length() {
        return Err(doido_core::anyhow::anyhow!(
            "password exceeds maximum length"
        ));
    }
    let secret = peppered_secret(password);
    let params = Params::new(
        config.memory_kib,
        config.iterations,
        config.parallelism,
        None,
    )
    .map_err(|e| doido_core::anyhow::anyhow!("invalid Argon2 params: {e}"))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2
        .hash_password(&secret, &salt)
        .map_err(|e| doido_core::anyhow::anyhow!("argon2 hash failed: {e}"))?;
    Ok(hash.to_string())
}

/// Hash with bcrypt at an explicit cost — **legacy / tests only** (creates `$2…` digests).
pub fn hash_password_with_cost(password: &str, cost: u32) -> Result<String> {
    bcrypt_hash(password, cost).map_err(|e| doido_core::anyhow::anyhow!("bcrypt hash failed: {e}"))
}

/// Verify a plaintext password against a PHC digest (Argon2id or bcrypt).
pub fn verify_password(password: &str, digest: &str) -> bool {
    with_password_serial(|| verify_password_inner(password, digest))
}

fn verify_password_inner(password: &str, digest: &str) -> bool {
    if digest.starts_with("$argon2") {
        let secret = peppered_secret(password);
        let Ok(parsed) = PasswordHash::new(digest) else {
            return false;
        };
        let Ok(params) = Params::try_from(&parsed) else {
            return false;
        };
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        return argon2.verify_password(&secret, &parsed).is_ok();
    }
    if digest.starts_with("$2") {
        return bcrypt_verify(password, digest).unwrap_or(false);
    }
    false
}

/// Whether `digest` should be replaced with a fresh Argon2id hash (bcrypt or weak Argon2 params).
pub fn needs_rehash(digest: &str) -> bool {
    with_password_serial(|| {
        if digest.starts_with("$2") {
            return true;
        }
        if !digest.starts_with("$argon2id$") {
            return true;
        }
        let Ok(parsed) = PasswordHash::new(digest) else {
            return true;
        };
        let Ok(params) = Params::try_from(&parsed) else {
            return true;
        };
        let target = active_config();
        params.m_cost() < target.memory_kib
            || params.t_cost() < target.iterations
            || params.p_cost() < target.parallelism
    })
}

/// Run a password verify against a fixed digest to equalise timing when no user exists.
pub fn dummy_password_verify(password: &str) {
    with_password_serial(|| {
        static DUMMY: OnceLock<String> = OnceLock::new();
        let digest = DUMMY.get_or_init(|| {
            hash_password_with_config_inner(
                "dummy-timing-mitigation",
                PasswordHashConfig::fast_test(),
            )
            .expect("dummy digest")
        });
        let _ = verify_password_inner(password, digest);
    });
}

/// Generate an unguessable token (256 bits, hex) for `has_secure_token` and
/// one-off tokens (password reset, etc.).
pub fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Gives a model that stores a password digest an `authenticate` method.
pub trait HasSecurePassword {
    /// The stored PHC digest (e.g. the `password_digest` column).
    fn password_digest(&self) -> &str;

    /// Return `true` when `password` matches the stored digest.
    fn authenticate(&self, password: &str) -> bool {
        verify_password(password, self.password_digest())
    }
}
