use doido_generators::commands::dbconsole::client_command;
use doido_generators::commands::runner::runner_command;

#[test]
fn dbconsole_resolves_the_client() {
    assert_eq!(
        client_command("sqlite://db/dev.db"),
        Some(("sqlite3".into(), vec!["db/dev.db".into()]))
    );
    assert_eq!(
        client_command("postgres://localhost/app"),
        Some(("psql".into(), vec!["postgres://localhost/app".into()]))
    );
    assert_eq!(client_command("mysql://x").unwrap().0, "mysql");
    assert!(client_command("mongodb://x").is_none());
}

#[test]
fn runner_runs_the_app_binary() {
    let (program, args) = runner_command(&["seed", "--force"]);
    assert_eq!(program, "cargo");
    assert_eq!(args, vec!["run", "--quiet", "--", "seed", "--force"]);
}
