//! End-to-end harness: build, migrate, boot server, run real interactions.

use std::path::PathBuf;
use std::time::Duration;

use super::cli::{self, doido};
use super::db;
use super::server;
use super::workspace::{self, BaseProfile};

pub struct RunningApp {
    pub base_url: String,
}

pub struct AppHarness {
    pub app: PathBuf,
    app_name: &'static str,
    port: u16,
}

impl AppHarness {
    pub fn new(scenario: &str, profile: BaseProfile) -> Self {
        let _lock = workspace::e2e_lock();
        let scenario_dir = workspace::fork_scenario(scenario, profile);
        cli::warm_base_profile(profile);
        let app = scenario_dir.join("blog");
        Self {
            app,
            app_name: "blog",
            port: server::free_port(),
        }
    }

    pub fn generate(&self, args: &[&str]) {
        doido(&self.app).args(args).assert().success();
    }

    pub fn configure_server(&self) {
        server::configure_port(&self.app, self.port);
    }

    pub fn build(&self) {
        cli::cargo_build(&self.app);
    }

    pub fn bin(&self) -> PathBuf {
        cli::app_bin(self.app_name, &self.app)
    }

    pub fn prepare_database(&self) {
        db::prepare_database(&self.bin(), &self.app);
    }

    pub fn run<F>(&self, interact: F)
    where
        F: FnOnce(RunningApp) + std::panic::UnwindSafe,
    {
        self.run_with_db(|_| {}, interact);
    }

    /// Full scenario: DB assertions after migrate, then server + interaction.
    pub fn run_with_db<F, G>(&self, after_migrate: F, interact: G)
    where
        F: FnOnce(&Self),
        G: FnOnce(RunningApp) + std::panic::UnwindSafe,
    {
        self.configure_server();
        self.build();
        self.prepare_database();
        after_migrate(self);

        let bin = self.bin();
        let base_url = format!("http://127.0.0.1:{}", self.port);
        let running = server::spawn(&bin, &self.app);
        server::wait_until_http_ok(&format!("{base_url}/"), Duration::from_secs(60));

        let app = RunningApp { base_url };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| interact(app)));
        running.shutdown();
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }
}
