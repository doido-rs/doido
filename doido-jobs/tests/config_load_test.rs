//! Config YAML parsing edge cases. Uses a process-wide lock for `set_current_dir` tests.

use doido_core::Environment;
use doido_jobs::config::{load, Backend, JobsConfig, JobsFileConfig};
use std::fs;
use std::sync::Mutex;
use tempfile::TempDir;

static CWD_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn from_yaml_parses_memory_jobs_settings() {
    let cfg = JobsFileConfig::from_yaml("jobs:\n  type: memory\n  concurrency: 2\n")
        .unwrap()
        .jobs
        .into_config();
    assert_eq!(cfg.backend, Backend::Memory);
    assert_eq!(cfg.concurrency, 2);
}

#[test]
fn from_yaml_rejects_invalid_yaml() {
    assert!(JobsFileConfig::from_yaml("jobs: [\n").is_err());
}

#[test]
fn empty_queues_in_yaml_falls_back_to_default() {
    let cfg = JobsFileConfig::from_yaml("jobs:\n  queues: []\n")
        .unwrap()
        .jobs
        .into_config();
    assert_eq!(cfg.queues, vec!["default"]);
}

#[test]
fn redis_settings_merge_with_defaults() {
    let cfg = JobsFileConfig::from_yaml("jobs:\n  type: redis\n  redis:\n    namespace: custom\n")
        .unwrap()
        .jobs
        .into_config();
    assert_eq!(cfg.backend, Backend::Redis);
    assert_eq!(cfg.redis_namespace, "custom");
    assert_eq!(cfg.redis_url, JobsConfig::default().redis_url);
}

#[test]
fn load_env_reads_jobs_yaml_from_disk() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: db\n  concurrency: 4\n",
    )
    .unwrap();

    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let cfg = JobsFileConfig::load_env(Environment::Test)
        .unwrap()
        .jobs
        .into_config();
    std::env::set_current_dir(original).unwrap();

    assert_eq!(cfg.backend, Backend::Db);
    assert_eq!(cfg.concurrency, 4);
}

#[test]
fn load_defaults_when_config_file_missing() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();

    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");
    let cfg = load();
    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }

    assert_eq!(cfg.backend, Backend::Memory);
    assert_eq!(cfg.concurrency, JobsConfig::default().concurrency);
}

#[test]
fn load_reads_existing_config_file() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  type: memory\n  concurrency: 9\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();

    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");
    let cfg = load();
    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }

    assert_eq!(cfg.concurrency, 9);
}

#[test]
fn jobs_settings_default_merges_into_config() {
    use doido_jobs::config::JobsSettings;

    let cfg = JobsSettings::default().into_config();
    assert_eq!(cfg.backend, Backend::Memory);
    assert_eq!(cfg.queues, vec!["default"]);
}

#[test]
fn jobs_file_config_load_reads_disk() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    fs::write(
        dir.path().join("config/test.yml"),
        "jobs:\n  concurrency: 11\n",
    )
    .unwrap();

    let original_dir = std::env::current_dir().unwrap();
    let original_env = std::env::var("DOIDO_ENV").ok();

    std::env::set_current_dir(dir.path()).unwrap();
    std::env::set_var("DOIDO_ENV", "test");
    let file = JobsFileConfig::load().unwrap();
    std::env::set_current_dir(original_dir).unwrap();
    if let Some(v) = original_env {
        std::env::set_var("DOIDO_ENV", v);
    } else {
        std::env::remove_var("DOIDO_ENV");
    }

    assert_eq!(file.jobs.into_config().concurrency, 11);
}

#[test]
fn load_env_errors_when_file_missing() {
    let _guard = CWD_LOCK.lock().unwrap();
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("config")).unwrap();
    let original = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    assert!(JobsFileConfig::load_env(Environment::Test).is_err());
    std::env::set_current_dir(original).unwrap();
}
