use crate::commands::write_files;
use crate::default_registry;
use crate::new_options::{
    parse_cache, parse_database, parse_jobs, CacheBackend, DatabaseBackend, JobsBackend,
};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

fn prompt_line(prompt: &str, default: &str) -> String {
    print!("{prompt} (default: {default}): ");
    io::stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read input");
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn prompt_database() -> DatabaseBackend {
    loop {
        let answer = prompt_line(
            "Which database? [sqlite/postgres/mysql]",
            DatabaseBackend::Sqlite.as_str(),
        );
        match parse_database(&answer) {
            Ok(db) => return db,
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn prompt_cache() -> CacheBackend {
    loop {
        let answer = prompt_line(
            "Which cache backend? [memory/redis/memcache]",
            CacheBackend::Memory.as_str(),
        );
        match parse_cache(&answer) {
            Ok(cache) => return cache,
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn prompt_jobs() -> JobsBackend {
    loop {
        let answer = prompt_line(
            "Which jobs backend? [memory/db/redis]",
            JobsBackend::Memory.as_str(),
        );
        match parse_jobs(&answer) {
            Ok(jobs) => return jobs,
            Err(e) => eprintln!("{e}"),
        }
    }
}

fn prompt_cable() -> bool {
    print!("Include doido-cable example channel? [y/N]: ");
    io::stdout().flush().expect("failed to flush stdout");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("failed to read input");
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

pub fn run_new(
    name: &str,
    non_interactive: bool,
    database: Option<DatabaseBackend>,
    cable: bool,
    cache: Option<CacheBackend>,
    jobs: Option<JobsBackend>,
) {
    let (database, cache, jobs, cable) = if non_interactive {
        (
            database.unwrap_or(DatabaseBackend::Sqlite),
            cache.unwrap_or(CacheBackend::Memory),
            jobs.unwrap_or(JobsBackend::Memory),
            cable,
        )
    } else {
        (
            database.unwrap_or_else(prompt_database),
            cache.unwrap_or_else(prompt_cache),
            jobs.unwrap_or_else(prompt_jobs),
            if cable { true } else { prompt_cable() },
        )
    };

    let registry = default_registry();
    let db_arg = format!("--database={}", database.as_str());
    let cache_arg = format!("--cache={}", cache.as_str());
    let jobs_arg = format!("--jobs={}", jobs.as_str());
    let mut new_args = vec![name, &db_arg, &cache_arg, &jobs_arg];
    if cable {
        new_args.push("--cable");
    }
    match registry.run("new", &new_args) {
        Ok(files) => {
            if let Err(e) = write_files(&files, Path::new(".")) {
                doido_core::tracing::error!("error writing files: {e}");
                std::process::exit(1);
            }
            let git_result = Command::new("git").args(["init", name]).output();
            match git_result {
                Ok(output) if output.status.success() => {
                    doido_core::tracing::info!("init {name}/.git");
                }
                _ => {
                    doido_core::tracing::warn!("git init failed. Run it manually: git init {name}")
                }
            }
            doido_core::tracing::info!("created '{name}'. Next: cd {name} && cargo build");
        }
        Err(e) => {
            doido_core::tracing::error!("{e}");
            std::process::exit(1);
        }
    }
}
