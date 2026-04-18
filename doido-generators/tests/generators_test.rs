use doido_generators::{default_registry, Generator,
    ControllerGenerator, ModelGenerator, MigrationGenerator,
    JobGenerator, MailerGenerator, ChannelGenerator, ScaffoldGenerator};

#[test]
fn test_controller_generator_produces_correct_file() {
    let files = ControllerGenerator.generate(&["Posts"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "app/controllers/posts_controller.rs");
    assert!(files[0].content.contains("PostsController"));
    assert!(files[0].content.contains("#[controller]"));
}

#[test]
fn test_model_generator_produces_correct_file() {
    let files = ModelGenerator.generate(&["User"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "app/models/user.rs");
    assert!(files[0].content.contains("DeriveEntityModel"));
}

#[test]
fn test_migration_generator_has_timestamp_in_filename() {
    let files = MigrationGenerator.generate(&["create_users"]).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].path.starts_with("db/migrations/"));
    assert!(files[0].path.ends_with("_create_users.rs"));
}

#[test]
fn test_job_generator_produces_correct_file() {
    let files = JobGenerator.generate(&["SendEmail"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "app/jobs/send_email_job.rs");
    assert!(files[0].content.contains("#[job"));
}

#[test]
fn test_mailer_generator_produces_correct_file() {
    let files = MailerGenerator.generate(&["Welcome"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "app/mailers/welcome_mailer.rs");
    assert!(files[0].content.contains("WelcomeMailer"));
}

#[test]
fn test_channel_generator_produces_correct_file() {
    let files = ChannelGenerator.generate(&["Chat"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "app/channels/chat_channel.rs");
    assert!(files[0].content.contains("ChatChannel"));
}

#[test]
fn test_scaffold_generator_produces_multiple_files() {
    let files = ScaffoldGenerator.generate(&["Post"]).unwrap();
    // Should produce: controller + model + migration + routes
    assert!(files.len() >= 4);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.iter().any(|p| p.contains("controller")));
    assert!(paths.iter().any(|p| p.contains("model")));
    assert!(paths.iter().any(|p| p.contains("migration")));
    assert!(paths.iter().any(|p| *p == "config/routes.rs"));
    // Route file should contain resources! call
    let routes = files.iter().find(|f| f.path == "config/routes.rs").unwrap();
    assert!(routes.content.contains("resources!(post)"));
}

#[test]
fn test_default_registry_has_all_generators() {
    let reg = default_registry();
    let names = reg.list();
    assert!(names.contains(&"controller"));
    assert!(names.contains(&"model"));
    assert!(names.contains(&"migration"));
    assert!(names.contains(&"job"));
    assert!(names.contains(&"mailer"));
    assert!(names.contains(&"channel"));
    assert!(names.contains(&"scaffold"));
}

#[test]
fn test_registry_runs_generator_by_name() {
    let reg = default_registry();
    let files = reg.run("controller", &["Admin"]).unwrap();
    assert_eq!(files[0].path, "app/controllers/admin_controller.rs");
}

#[test]
fn test_generator_missing_arg_returns_error() {
    let result = ControllerGenerator.generate(&[]);
    assert!(result.is_err());
}

#[test]
fn test_registry_unknown_generator_returns_error() {
    let reg = default_registry();
    assert!(reg.run("nonexistent", &[]).is_err());
}
