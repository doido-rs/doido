use doido_generators::{
    default_registry, ChannelGenerator, ControllerGenerator, Generator, JobGenerator,
    MailerGenerator, MigrationGenerator, ModelGenerator, ScaffoldGenerator,
};

#[test]
fn test_controller_generator_produces_correct_file() {
    let files = ControllerGenerator.generate(&["Posts"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/controllers/posts_controller.rs");
    assert!(files[0].content.contains("PostsController"));
    assert!(files[0].content.contains("#[controller]"));
}

#[test]
fn test_model_generator_produces_model_migration_and_updates_lib() {
    let files = ModelGenerator.generate(&["User"]).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    // Model file in app/models/.
    assert!(paths.contains(&"app/models/user.rs"));
    let model = files.iter().find(|f| f.path == "app/models/user.rs").unwrap();
    assert!(model.content.contains("DeriveEntityModel"));
    assert!(model.content.contains("table_name = \"users\""));

    // Migration file in db/migration/src/.
    assert!(paths
        .iter()
        .any(|p| p.starts_with("db/migration/src/m") && p.ends_with("_create_users_table.rs")));

    // Migration crate lib.rs is updated to register the new migration.
    let lib = files
        .iter()
        .find(|f| f.path == "db/migration/src/lib.rs")
        .unwrap();
    assert!(lib.content.contains("mod m"));
    assert!(lib.content.contains("_create_users_table;"));
    assert!(lib.content.contains("Box::new(m"));
    assert!(lib.content.contains("_create_users_table::Migration),"));
}

#[test]
fn test_model_generator_pluralizes_irregular_table_name() {
    // Proves the model generator uses the inflector (Person -> people), not a
    // naive `name + "s"` ("persons").
    let files = ModelGenerator.generate(&["Person"]).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    assert!(paths.contains(&"app/models/person.rs"));
    let model = files
        .iter()
        .find(|f| f.path == "app/models/person.rs")
        .unwrap();
    assert!(model.content.contains("table_name = \"people\""));
    assert!(paths
        .iter()
        .any(|p| p.ends_with("_create_people_table.rs")));
}

#[test]
fn test_model_generator_emits_columns_from_field_specs() {
    let files = ModelGenerator
        .generate(&[
            "Post",
            "title:string:not_null",
            "body:text",
            "author:references",
            "slug:string:unique",
            "views:integer:index",
        ])
        .unwrap();

    // Model struct carries one field per column with correct nullability.
    let model = files
        .iter()
        .find(|f| f.path == "app/models/post.rs")
        .unwrap();
    assert!(model.content.contains("pub title: String,"));
    assert!(model.content.contains("pub body: Option<String>,"));
    assert!(model.content.contains("pub author_id: i64,"));
    assert!(model.content.contains("pub slug: Option<String>,"));
    assert!(model.content.contains("pub views: Option<i32>,"));

    // Migration builds the columns and an index for the `:index` field.
    let migration = files
        .iter()
        .find(|f| f.path.ends_with("_create_posts_table.rs"))
        .unwrap();
    assert!(migration.content.contains("t.string(\"title\").not_null();"));
    assert!(migration.content.contains("t.text(\"body\");"));
    assert!(migration.content.contains("t.references(\"author\");"));
    assert!(migration
        .content
        .contains("t.string(\"slug\").unique_key();"));
    assert!(migration.content.contains("t.integer(\"views\");"));
    assert!(migration.content.contains("use doido_model::migration::{add_index, create_table, drop_table};"));
    assert!(migration
        .content
        .contains("add_index(manager, \"posts\", &[\"views\"]).await?;"));
}

#[test]
fn test_model_generator_without_fields_emits_empty_table() {
    let files = ModelGenerator.generate(&["User"]).unwrap();
    let migration = files
        .iter()
        .find(|f| f.path.ends_with("_create_users_table.rs"))
        .unwrap();
    // Falls back to the no-column closure (and no unused `add_index` import).
    assert!(migration.content.contains("|_t| {}).await"));
    assert!(migration
        .content
        .contains("use doido_model::migration::{create_table, drop_table};"));
    assert!(!migration.content.contains("add_index"));
}

#[test]
fn test_model_generator_rejects_bad_field_type() {
    assert!(ModelGenerator.generate(&["User", "age:notatype"]).is_err());
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
    assert!(paths.contains(&"config/routes.rs"));
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
    assert_eq!(files[0].path, "src/controllers/admin_controller.rs");
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
