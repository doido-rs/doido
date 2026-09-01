//! `doido credentials` — AES-256-GCM encrypted credentials (Rails
//! `credentials`). Plaintext lives only in `$EDITOR`; on disk it is the
//! encrypted `config/credentials.yml.enc`, decryptable with the app's master
//! key (`config/master.key` or the `DOIDO_MASTER_KEY` env var).

use crate::anyhow::anyhow;
use crate::Result;
use clap::Subcommand;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const CONFIG_DIR: &str = "config";
const MASTER_KEY_NAME: &str = "master.key";
const CREDENTIALS_NAME: &str = "credentials.yml.enc";

/// Seed contents shown when editing credentials for the first time.
const DEFAULT_TEMPLATE: &str = "# Encrypted credentials — edit with `doido credentials edit`.\n\
# secret_key_base: change-me\n\
# aws:\n\
#   access_key_id: ...\n\
#   secret_access_key: ...\n";

#[derive(Subcommand)]
pub enum CredentialsCommand {
    /// Edit the encrypted credentials in `$EDITOR`
    Edit,
    /// Print the decrypted credentials to stdout
    Show,
}

pub fn run(cmd: CredentialsCommand) {
    let config_dir = Path::new(CONFIG_DIR);
    let result = match cmd {
        CredentialsCommand::Edit => edit(config_dir),
        CredentialsCommand::Show => show(config_dir),
    };
    if let Err(e) = result {
        crate::tracing::error!("credentials: {e}");
        std::process::exit(1);
    }
}

fn master_key_path(config_dir: &Path) -> PathBuf {
    config_dir.join(MASTER_KEY_NAME)
}

fn credentials_path(config_dir: &Path) -> PathBuf {
    config_dir.join(CREDENTIALS_NAME)
}

/// The master key from `DOIDO_MASTER_KEY` or `config/master.key`, if available.
fn read_master_key(config_dir: &Path) -> Option<Vec<u8>> {
    if let Ok(key) = std::env::var("DOIDO_MASTER_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            return Some(key.as_bytes().to_vec());
        }
    }
    fs::read_to_string(master_key_path(config_dir))
        .ok()
        .map(|s| s.trim().as_bytes().to_vec())
}

/// The master key, generating (and gitignoring) `config/master.key` if missing.
fn ensure_master_key(config_dir: &Path) -> Result<Vec<u8>> {
    if let Some(key) = read_master_key(config_dir) {
        return Ok(key);
    }
    let key = crate::crypto::generate_key_hex();
    fs::create_dir_all(config_dir).map_err(|e| anyhow!("could not create {CONFIG_DIR}: {e}"))?;
    fs::write(master_key_path(config_dir), &key)
        .map_err(|e| anyhow!("could not write master key: {e}"))?;
    gitignore_master_key();
    crate::tracing::info!(
        "generated {}/{MASTER_KEY_NAME} (keep it secret — it was added to .gitignore)",
        config_dir.display()
    );
    Ok(key.into_bytes())
}

/// The master key, erroring if none is configured (for read-only commands).
fn require_master_key(config_dir: &Path) -> Result<Vec<u8>> {
    read_master_key(config_dir).ok_or_else(|| {
        anyhow!(
            "no master key: set DOIDO_MASTER_KEY or create {}/{MASTER_KEY_NAME} \
             (run `doido credentials edit`)",
            CONFIG_DIR
        )
    })
}

/// Ensure `config/master.key` is gitignored (append the line if absent).
fn gitignore_master_key() {
    let path = Path::new(".gitignore");
    let mut content = fs::read_to_string(path).unwrap_or_default();
    if content.contains(MASTER_KEY_NAME) {
        return;
    }
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("config/master.key\n");
    let _ = fs::write(path, content);
}

/// Decrypt the credentials file, or return the seed template when absent.
fn read_credentials(config_dir: &Path, key: &[u8]) -> Result<String> {
    match fs::read(credentials_path(config_dir)) {
        Ok(blob) => {
            let plain = crate::crypto::decrypt(key, &blob).ok_or_else(|| {
                anyhow!("could not decrypt {CREDENTIALS_NAME} (wrong master key?)")
            })?;
            String::from_utf8(plain).map_err(|e| anyhow!("credentials are not valid UTF-8: {e}"))
        }
        Err(_) => Ok(DEFAULT_TEMPLATE.to_string()),
    }
}

/// Encrypt `plaintext` to the credentials file.
fn write_credentials(config_dir: &Path, key: &[u8], plaintext: &str) -> Result<()> {
    fs::create_dir_all(config_dir).map_err(|e| anyhow!("could not create {CONFIG_DIR}: {e}"))?;
    let blob = crate::crypto::encrypt(key, plaintext.as_bytes());
    fs::write(credentials_path(config_dir), blob)
        .map_err(|e| anyhow!("could not write {CREDENTIALS_NAME}: {e}"))
}

fn edit(config_dir: &Path) -> Result<()> {
    let key = ensure_master_key(config_dir)?;
    let current = read_credentials(config_dir, &key)?;

    let tmp = std::env::temp_dir().join(format!("doido-credentials-{}.yml", std::process::id()));
    fs::write(&tmp, &current).map_err(|e| anyhow!("could not write temp file: {e}"))?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor).arg(&tmp).status();
    let edited = match status {
        Ok(s) if s.success() => {
            fs::read_to_string(&tmp).map_err(|e| anyhow!("could not read temp file: {e}"))?
        }
        Ok(_) => {
            let _ = fs::remove_file(&tmp);
            return Err(anyhow!("editor exited non-zero; credentials unchanged"));
        }
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            return Err(anyhow!("could not launch editor '{editor}': {e}"));
        }
    };
    let _ = fs::remove_file(&tmp);

    write_credentials(config_dir, &key, &edited)?;
    crate::tracing::info!(
        "credentials saved to {}",
        credentials_path(config_dir).display()
    );
    Ok(())
}

fn show(config_dir: &Path) -> Result<()> {
    let key = require_master_key(config_dir)?;
    let plain = read_credentials(config_dir, &key)?;
    print!("{plain}");
    let _ = std::io::stdout().flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialises tests that read/write the process-global `DOIDO_MASTER_KEY`
    /// env var (and the current dir); without this they race under the parallel
    /// test runner — e.g. one test's `DOIDO_MASTER_KEY` leaks into another's
    /// `read_master_key`, which checks the env before the key file.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn temp_dir() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("doido-cred-{}", crate::crypto::generate_key_hex()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn credentials_round_trip_through_files() {
        let dir = temp_dir();
        let key = b"a-test-master-key";

        write_credentials(&dir, key, "secret_key_base: abc123\n").unwrap();
        assert_eq!(
            read_credentials(&dir, key).unwrap(),
            "secret_key_base: abc123\n"
        );

        // A wrong key fails authentication.
        assert!(read_credentials(&dir, b"wrong-key").is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn absent_credentials_yield_the_seed_template() {
        let dir = temp_dir();
        let plain = read_credentials(&dir, b"k").unwrap();
        assert!(plain.contains("Encrypted credentials"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ensure_master_key_writes_and_gitignores() {
        let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp_dir();
        let project = dir.join("proj");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join(".gitignore"), "existing\n").unwrap();
        let original = std::env::current_dir().unwrap();
        let original_key = std::env::var("DOIDO_MASTER_KEY").ok();
        std::env::remove_var("DOIDO_MASTER_KEY");
        std::env::set_current_dir(&project).unwrap();
        let key = ensure_master_key(Path::new("config")).unwrap();
        assert!(!key.is_empty());
        assert!(master_key_path(Path::new("config")).is_file());
        let gitignore = fs::read_to_string(".gitignore").unwrap();
        assert!(gitignore.contains("config/master.key"));
        std::env::set_current_dir(original).unwrap();
        if let Some(v) = original_key {
            std::env::set_var("DOIDO_MASTER_KEY", v);
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_master_key_from_env() {
        let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp_dir();
        let original = std::env::var("DOIDO_MASTER_KEY").ok();
        std::env::set_var("DOIDO_MASTER_KEY", "env-master-key");
        assert_eq!(
            read_master_key(Path::new("config")).unwrap(),
            b"env-master-key"
        );
        if let Some(v) = original {
            std::env::set_var("DOIDO_MASTER_KEY", v);
        } else {
            std::env::remove_var("DOIDO_MASTER_KEY");
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn show_prints_decrypted_credentials() {
        let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original_key = std::env::var("DOIDO_MASTER_KEY").ok();
        // `show` → `require_master_key` checks the env before the key file; unset
        // it so the on-disk `config/master.key` we just wrote is used.
        std::env::remove_var("DOIDO_MASTER_KEY");
        let dir = temp_dir();
        let config = dir.join("config");
        fs::create_dir_all(&config).unwrap();
        fs::write(config.join(MASTER_KEY_NAME), "show-key").unwrap();
        write_credentials(&config, b"show-key", "api_token: secret\n").unwrap();
        show(&config).unwrap();
        if let Some(v) = original_key {
            std::env::set_var("DOIDO_MASTER_KEY", v);
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn require_master_key_errors_when_missing() {
        let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = temp_dir();
        let config = dir.join("config");
        fs::create_dir_all(&config).unwrap();
        let original_key = std::env::var("DOIDO_MASTER_KEY").ok();
        std::env::remove_var("DOIDO_MASTER_KEY");
        assert!(require_master_key(&config).is_err());
        if let Some(v) = original_key {
            std::env::set_var("DOIDO_MASTER_KEY", v);
        }
        fs::remove_dir_all(&dir).ok();
    }
}
