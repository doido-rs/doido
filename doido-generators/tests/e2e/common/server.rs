//! Spawn the generated app server and wait for readiness.

use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct RunningServer {
    child: Child,
}

impl RunningServer {
    pub fn shutdown(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub fn configure_port(app: &Path, port: u16) {
    let dev_yml = app.join("config/development.yml");
    let cfg = std::fs::read_to_string(&dev_yml).unwrap();
    let cfg = cfg
        .replace("bind: 0.0.0.0", "bind: 127.0.0.1")
        .replace("port: 3000", &format!("port: {port}"));
    std::fs::write(dev_yml, cfg).unwrap();
}

pub fn spawn(bin: &Path, app: &Path) -> RunningServer {
    let child = Command::new(bin)
        .arg("server")
        .current_dir(app)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn server");
    RunningServer { child }
}

pub fn wait_until_http_ok(url: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if ureq::get(url).call().is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("server did not become ready at {url} within {:?}", timeout);
}
