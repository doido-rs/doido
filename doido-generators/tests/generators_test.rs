use doido_generators::{
    default_registry, ChannelGenerator, ControllerGenerator, Generator, HelperGenerator,
    JobGenerator, MailerGenerator, MigrationGenerator, ModelGenerator, ScaffoldGenerator,
};

#[test]
fn test_controller_generator_produces_correct_file() {
    let files = ControllerGenerator.generate(&["Posts"]).unwrap();
    let ctrl = files
        .iter()
        .find(|f| f.path == "app/controllers/posts_controller.rs")
        .unwrap();
    assert!(ctrl.content.contains("PostsController"));
    assert!(ctrl.content.contains("#[controller]"));
    assert!(ctrl.content.contains("use crate::helpers::PostsHelper"));
    assert!(ctrl.content.contains("PostsHelper::index_count"));
    let helper = files
        .iter()
        .find(|f| f.path == "app/helpers/posts_helper.rs")
        .unwrap();
    assert!(helper.content.contains("PostsHelper"));
    assert!(helper.content.contains("#[helper]"));
    let helpers_mod = files
        .iter()
        .find(|f| f.path == "app/helpers/mod.rs")
        .unwrap();
    assert!(helpers_mod
        .content
        .contains("pub use posts_helper::PostsHelper;"));
    let routes = files.iter().find(|f| f.path == "config/routes.rs").unwrap();
    assert!(routes
        .content
        .contains("get!(\"/posts\", PostsController::index);"));
    assert!(routes
        .content
        .contains("use crate::controllers::PostsController;"));
    // A TODO test stub is emitted alongside it.
    let test = files
        .iter()
        .find(|f| f.path == "tests/posts_controller_test.rs")
        .expect("controller test stub emitted");
    assert!(test.content.contains("fn posts_controller_todo()"));
}

#[test]
fn test_model_generator_produces_model_migration_and_updates_lib() {
    let files = ModelGenerator.generate(&["User"]).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    // Entity file in app/models/_entities/; extension stub in app/models/.
    assert!(paths.contains(&"app/models/_entities/users.rs"));
    assert!(paths.contains(&"app/models/user.rs"));
    let entity = files
        .iter()
        .find(|f| f.path == "app/models/_entities/users.rs")
        .unwrap();
    assert!(entity.content.contains("DeriveEntityModel"));
    assert!(entity.content.contains("table_name = \"users\""));
    assert!(!entity
        .content
        .contains("impl ActiveModelBehavior for ActiveModel {}"));
    let extension = files
        .iter()
        .find(|f| f.path == "app/models/user.rs")
        .unwrap();
    assert!(extension
        .content
        .contains("pub use super::_entities::users::*;"));
    assert!(extension
        .content
        .contains("impl ActiveModelBehavior for ActiveModel {}"));

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

    let migration = files
        .iter()
        .find(|f| f.path.ends_with("_create_users_table.rs"))
        .unwrap();
    let module = migration
        .path
        .strip_prefix("db/migration/src/")
        .unwrap()
        .strip_suffix(".rs")
        .unwrap();
    assert!(migration
        .content
        .contains("impl MigrationName for Migration"));
    assert!(!migration.content.contains("DeriveMigrationName"));
    assert!(migration.content.contains(&format!("\"{module}\"")));
}

#[test]
fn test_model_generator_pluralizes_irregular_table_name() {
    // Proves the model generator uses the inflector (Person -> people), not a
    // naive `name + "s"` ("persons").
    let files = ModelGenerator.generate(&["Person"]).unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    assert!(paths.contains(&"app/models/person.rs"));
    assert!(paths.contains(&"app/models/_entities/people.rs"));
    let entity = files
        .iter()
        .find(|f| f.path == "app/models/_entities/people.rs")
        .unwrap();
    assert!(entity.content.contains("table_name = \"people\""));
    assert!(paths.iter().any(|p| p.ends_with("_create_people_table.rs")));
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

    // Entity struct carries one field per column with correct nullability.
    let entity = files
        .iter()
        .find(|f| f.path == "app/models/_entities/posts.rs")
        .unwrap();
    assert!(entity.content.contains("pub title: String,"));
    assert!(entity.content.contains("pub body: Option<String>,"));
    assert!(entity.content.contains("pub author_id: i64,"));
    assert!(entity.content.contains("pub slug: Option<String>,"));
    assert!(entity.content.contains("pub views: Option<i32>,"));

    // Migration builds the columns and an index for the `:index` field.
    let migration = files
        .iter()
        .find(|f| f.path.ends_with("_create_posts_table.rs"))
        .unwrap();
    assert!(migration
        .content
        .contains("t.string(\"title\").not_null();"));
    assert!(migration.content.contains("t.text(\"body\");"));
    assert!(migration.content.contains("t.references(\"author\");"));
    assert!(migration
        .content
        .contains("t.string(\"slug\").unique_key();"));
    assert!(migration.content.contains("t.integer(\"views\");"));
    assert!(migration
        .content
        .contains("use doido::model::migration::{add_index, create_table, drop_table};"));
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
        .contains("use doido::model::migration::{create_table, drop_table};"));
    assert!(!migration.content.contains("add_index"));
}

#[test]
fn test_model_generator_rejects_bad_field_type() {
    assert!(ModelGenerator.generate(&["User", "age:notatype"]).is_err());
}

#[test]
fn test_model_generator_emits_test_stub() {
    let files = ModelGenerator.generate(&["User"]).unwrap();
    let test = files
        .iter()
        .find(|f| f.path == "tests/user_model_test.rs")
        .expect("model test stub emitted");
    assert!(test.content.contains("TODO"));
    assert!(test.content.contains("fn user_model_todo()"));
}

#[test]
fn test_scaffold_emits_model_test_and_controller_request_tests() {
    let files = ScaffoldGenerator
        .generate(&["Post", "title:string"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    // Model test stub is emitted alongside the model.
    assert!(paths.contains(&"tests/post_model_test.rs"));

    // The controller file itself no longer carries an in-file test module.
    let ctrl = files
        .iter()
        .find(|f| f.path == "app/controllers/posts_controller.rs")
        .unwrap();
    assert!(!ctrl.content.contains("#[cfg(test)]"));

    // A separate tests/ file holds one test per controller action.
    let test = files
        .iter()
        .find(|f| f.path == "tests/posts_controller_test.rs")
        .expect("controller test file emitted");
    assert!(test.content.contains("#[path = \"../config/routes.rs\"]")); // builds the router
    assert!(test.content.contains("pool::test_lock()")); // serialized via shared lock
    assert!(test.content.contains("title=Test")); // sample form body
    for action in [
        "index_request",
        "new_request",
        "create_request",
        "show_request",
        "edit_request",
        "update_request",
        "destroy_request",
    ] {
        assert!(
            test.content.contains(&format!("async fn {action}()")),
            "missing per-action test {action}"
        );
    }
    assert!(test.content.contains("StatusCode::OK"));
    assert!(test.content.contains("StatusCode::FOUND"));
}

#[test]
fn test_migration_generator_create_pattern_uses_doido_model() {
    let files = MigrationGenerator
        .generate(&["create_users", "email:string:unique"])
        .unwrap();
    // Written into the migration crate (not the old `db/migrations/` dir).
    let mig = files
        .iter()
        .find(|f| f.path.starts_with("db/migration/src/") && f.path.ends_with("_create_users.rs"))
        .unwrap();
    // Uses the doido-model builders, not raw sea-orm.
    assert!(mig
        .content
        .contains("create_table(manager, \"users\", |t| {"));
    assert!(mig.content.contains("t.string(\"email\").unique_key();"));
    assert!(mig.content.contains("drop_table(manager, \"users\").await"));
    assert!(mig
        .content
        .contains("use doido::model::migration::{create_table, drop_table};"));
    assert!(!mig.content.contains("Table::create()"));
    assert!(!mig.content.contains("ColumnDef"));
    let module = mig
        .path
        .strip_prefix("db/migration/src/")
        .unwrap()
        .strip_suffix(".rs")
        .unwrap();
    assert!(mig.content.contains("impl MigrationName for Migration"));
    assert!(!mig.content.contains("DeriveMigrationName"));
    assert!(mig.content.contains(&format!("\"{module}\"")));
    // Registered in the Migrator's lib.rs, and a test stub is scaffolded.
    let lib = files
        .iter()
        .find(|f| f.path == "db/migration/src/lib.rs")
        .unwrap();
    assert!(lib.content.contains("_create_users::Migration)"));
    assert!(files
        .iter()
        .any(|f| f.path == "tests/create_users_migration_test.rs"));
}

#[test]
fn test_migration_generator_add_columns_pattern() {
    let files = MigrationGenerator
        .generate(&["add_slug_to_posts", "slug:string:unique"])
        .unwrap();
    let mig = files
        .iter()
        .find(|f| f.path.ends_with("_add_slug_to_posts.rs"))
        .unwrap();
    assert!(mig
        .content
        .contains("use doido::model::migration::alter_table;"));
    assert!(mig
        .content
        .contains("alter_table(manager, \"posts\", |t| {"));
    assert!(mig
        .content
        .contains("t.add_column(\"slug\", |c| { c.string().unique_key(); });"));
    // `down` reverses the change by dropping the column.
    assert!(mig.content.contains("t.drop_column(\"slug\");"));
}

#[test]
fn test_migration_generator_remove_columns_pattern() {
    let files = MigrationGenerator
        .generate(&["remove_slug_from_posts", "slug:string"])
        .unwrap();
    let mig = files
        .iter()
        .find(|f| f.path.ends_with("_remove_slug_from_posts.rs"))
        .unwrap();
    // up drops, down re-adds.
    assert!(mig.content.contains("t.drop_column(\"slug\");"));
    assert!(mig
        .content
        .contains("t.add_column(\"slug\", |c| { c.string(); });"));
}

#[test]
fn test_migration_generator_generic_name_uses_doido_model_skeleton() {
    let files = MigrationGenerator.generate(&["backfill_stuff"]).unwrap();
    let mig = files
        .iter()
        .find(|f| f.path.ends_with("_backfill_stuff.rs"))
        .unwrap();
    assert!(mig
        .content
        .contains("create_table(manager, \"backfill_stuff\", |_t| {})"));
    assert!(!mig.content.contains("Table::create()"));
    assert!(!mig.content.contains("ColumnDef"));
    assert!(!mig.content.contains("Alias::new"));
}

#[test]
fn test_job_generator_produces_correct_file() {
    let files = JobGenerator.generate(&["SendEmail"]).unwrap();
    let job = files
        .iter()
        .find(|f| f.path == "app/jobs/send_email_job.rs")
        .unwrap();
    assert!(job.content.contains("#[job"));
    // The context the engine carries is the job's first parameter, followed by a
    // typed payload.
    assert!(job
        .content
        .contains("async fn send_email_job(ctx: &JobContext, payload: SendEmailPayload)"));
    // The payload type is (de)serializable so the queue can persist it as JSON.
    assert!(job.content.contains("pub struct SendEmailPayload"));
    assert!(job.content.contains("Serialize"));
    assert!(job.content.contains("Deserialize"));
    let test = files
        .iter()
        .find(|f| f.path == "tests/send_email_job_test.rs")
        .expect("job test stub emitted");
    assert!(test.content.contains("fn send_email_job_todo()"));
}

#[test]
fn test_helper_generator_produces_correct_file() {
    let files = HelperGenerator.generate(&["Posts"]).unwrap();
    let helper = files
        .iter()
        .find(|f| f.path == "app/helpers/posts_helper.rs")
        .unwrap();
    assert!(helper.content.contains("PostsHelper"));
    assert!(helper.content.contains("#[helper]"));
    let helpers_mod = files
        .iter()
        .find(|f| f.path == "app/helpers/mod.rs")
        .unwrap();
    assert!(helpers_mod.content.contains("pub mod posts_helper;"));
    assert!(helpers_mod
        .content
        .contains("pub use posts_helper::PostsHelper;"));
    let test = files
        .iter()
        .find(|f| f.path == "tests/posts_helper_test.rs")
        .expect("helper test stub emitted");
    assert!(test.content.contains("fn posts_helper_todo()"));
}

#[test]
fn test_mailer_generator_produces_correct_file() {
    let files = MailerGenerator.generate(&["Welcome"]).unwrap();
    let mailer = files
        .iter()
        .find(|f| f.path == "app/mailers/welcome_mailer.rs")
        .unwrap();
    assert!(mailer.content.contains("WelcomeMailer"));
    assert!(files
        .iter()
        .any(|f| f.path == "tests/welcome_mailer_test.rs"));
}

#[test]
fn test_channel_generator_produces_correct_file() {
    let files = ChannelGenerator.generate(&["Chat"]).unwrap();
    let channel = files
        .iter()
        .find(|f| f.path == "app/channels/chat_channel.rs")
        .unwrap();
    assert!(channel.content.contains("ChatChannel"));
    assert!(files.iter().any(|f| f.path == "tests/chat_channel_test.rs"));
}

#[test]
fn test_scaffold_generator_produces_multiple_files() {
    let files = ScaffoldGenerator
        .generate(&["Post", "title:string:not_null", "body:text"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    // Model + migration + controller + 5 views + routes + both mod.rs registries.
    assert!(paths.contains(&"app/models/post.rs"));
    assert!(paths.contains(&"app/controllers/posts_controller.rs"));
    assert!(paths.contains(&"app/helpers/posts_helper.rs"));
    assert!(paths.contains(&"app/helpers/mod.rs"));
    assert!(paths.iter().any(|p| p.ends_with("_create_posts_table.rs")));
    assert!(paths.contains(&"app/views/posts/index.html.tera"));
    assert!(paths.contains(&"app/views/posts/show.html.tera"));
    assert!(paths.contains(&"app/views/posts/new.html.tera"));
    assert!(paths.contains(&"app/views/posts/edit.html.tera"));
    assert!(paths.contains(&"app/views/posts/_form.html.tera"));
    assert!(paths.contains(&"config/routes.rs"));

    // Controller has all 7 RESTful actions and a strong-params struct.
    let ctrl = files
        .iter()
        .find(|f| f.path == "app/controllers/posts_controller.rs")
        .unwrap();
    for action in [
        "index", "show", "new", "create", "edit", "update", "destroy",
    ] {
        assert!(
            ctrl.content.contains(&format!("fn {action}(")),
            "missing action {action}"
        );
    }
    assert!(ctrl.content.contains("pub struct PostsController;"));
    assert!(ctrl.content.contains("use crate::helpers::PostsHelper"));
    assert!(ctrl.content.contains("PostsHelper::index_count"));
    assert!(ctrl.content.contains("pub struct PostForm"));
    assert!(ctrl.content.contains("pub title: String,"));
    assert!(ctrl.content.contains("title: Set(form.title),"));
    assert!(ctrl.content.contains("posts/index"));

    // Route injection (plural + controller), preserving the existing route.
    let routes = files.iter().find(|f| f.path == "config/routes.rs").unwrap();
    assert!(routes
        .content
        .contains("resources!(posts, PostsController);"));
    assert!(routes
        .content
        .contains("use crate::controllers::PostsController;"));
    assert!(routes.content.contains("HelloController::index")); // preserved

    // Controller registered in app/controllers/mod.rs.
    let cmod = files
        .iter()
        .find(|f| f.path == "app/controllers/mod.rs")
        .unwrap();
    assert!(cmod.content.contains("mod posts_controller;"));
    assert!(cmod
        .content
        .contains("pub use posts_controller::PostsController;"));

    let helper = files
        .iter()
        .find(|f| f.path == "app/helpers/posts_helper.rs")
        .unwrap();
    assert!(helper.content.contains("PostsHelper"));
    assert!(helper.content.contains("fn index_count"));

    let hmod = files
        .iter()
        .find(|f| f.path == "app/helpers/mod.rs")
        .unwrap();
    assert!(hmod.content.contains("pub mod posts_helper;"));
    assert!(hmod.content.contains("pub use posts_helper::PostsHelper;"));

    // Views are field-driven.
    let index = files
        .iter()
        .find(|f| f.path == "app/views/posts/index.html.tera")
        .unwrap();
    assert!(index.content.contains("<th>title</th>"));
    assert!(index.content.contains("{% for post in posts %}"));
    assert!(index.content.contains("{{ post.title }}"));
    // Delete button issues a DELETE to the destroy route for each row.
    assert!(index.content.contains(">Delete</button>"));
    assert!(index
        .content
        .contains("fetch('/posts/{{ post.id }}', { method: 'DELETE' })"));
}

#[test]
fn test_scaffold_boolean_field_form_and_params() {
    let files = ScaffoldGenerator
        .generate(&[
            "Post",
            "title:string:not_null",
            "published:boolean:not_null",
        ])
        .unwrap();

    let ctrl = files
        .iter()
        .find(|f| f.path == "app/controllers/posts_controller.rs")
        .unwrap();
    assert!(ctrl.content.contains("#[serde(default)]"));
    assert!(ctrl.content.contains("pub published: bool,"));
    assert!(ctrl.content.contains("published: Set(form.published),"));

    let form = files
        .iter()
        .find(|f| f.path == "app/views/posts/_form.html.tera")
        .unwrap();
    assert!(form.content.contains("type=\"checkbox\""));
    assert!(form.content.contains("name=\"published\""));
    assert!(form.content.contains("value=\"true\""));
    assert!(form
        .content
        .contains("{% if post is defined and post.published %} checked{% endif %}"));
}

#[test]
fn test_scaffold_api_emits_json_controller_and_no_views() {
    let files = ScaffoldGenerator
        .generate(&["Post", "title:string", "--api"])
        .unwrap();
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();

    // No view templates in API mode.
    assert!(!paths.iter().any(|p| p.contains("app/views/")));

    let ctrl = files
        .iter()
        .find(|f| f.path == "app/controllers/posts_controller.rs")
        .unwrap();
    assert!(ctrl.content.contains("ctx.json("));
    assert!(ctrl.content.contains("use crate::helpers::PostsHelper"));
    assert!(ctrl.content.contains("ctx.body_json()"));
    assert!(!ctrl.content.contains("ctx.render("));

    // API controllers have no new/edit form actions, so the injected route must
    // exclude them — otherwise `resources!` references methods that don't exist.
    let routes = files.iter().find(|f| f.path == "config/routes.rs").unwrap();
    assert!(routes
        .content
        .contains("resources!(posts, PostsController, except: [new, edit]);"));
}

#[test]
fn test_default_registry_has_all_generators() {
    let reg = default_registry();
    let names = reg.list();
    assert!(names.contains(&"controller"));
    assert!(names.contains(&"helper"));
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
    assert!(files
        .iter()
        .any(|f| f.path == "app/controllers/admin_controller.rs"));
    assert!(files
        .iter()
        .any(|f| f.path == "app/helpers/admin_helper.rs"));
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

// --- App-installed custom generators (doido::Doido::register_generator) ---

use doido_generators::commands::generate::registry_for_project_at_with;
use doido_generators::GeneratedFile;

/// A generator defined outside `doido-generators`, as an app would define one.
struct GreeterGenerator;

impl Generator for GreeterGenerator {
    fn name(&self) -> &str {
        "greeter"
    }

    fn generate(&self, args: &[&str]) -> doido_core::Result<Vec<GeneratedFile>> {
        let name = args.first().copied().unwrap_or("World");
        Ok(vec![GeneratedFile {
            path: format!("lib/{}.rs", name.to_lowercase()),
            content: format!("// hello {name}\n"),
        }])
    }
}

#[test]
fn test_custom_generator_registers_lists_and_dispatches() {
    // `dir` has no Cargo.toml, so only built-ins + the app-supplied extra apply.
    let dir = tempfile::tempdir().unwrap();
    let reg = registry_for_project_at_with(dir.path(), vec![Box::new(GreeterGenerator)]);

    // The custom generator is listed alongside the framework built-ins.
    let names = reg.list();
    assert!(names.contains(&"greeter"), "custom generator not listed");
    assert!(names.contains(&"controller"), "built-ins still present");

    // ...and dispatches by name exactly like a built-in.
    let files = reg.run("greeter", &["Foo"]).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "lib/foo.rs");
    assert!(files[0].content.contains("hello Foo"));
}

// --- Template overrides + project generators ---

use doido_generators::{project_generator, templates, GeneratorGenerator, TemplatesGenerator};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_resolve_prefers_project_override() {
    let dir = tempdir().unwrap();
    let rel = "controller/controller.rs.template";
    // No override file yet → built-in default is returned.
    let default = templates::get_with_root(dir.path(), rel);
    assert!(default.contains("{pascal}Controller"));

    // Write a project override; now it wins.
    fs::create_dir_all(dir.path().join("controller")).unwrap();
    fs::write(dir.path().join(rel), "OVERRIDDEN {pascal}").unwrap();
    let overridden = templates::get_with_root(dir.path(), rel);
    assert_eq!(overridden, "OVERRIDDEN {pascal}");
}

#[test]
fn test_templates_generator_ejects_all_and_filtered() {
    // No arg → ejects everything under templates/.
    let all = TemplatesGenerator.generate(&[]).unwrap();
    assert!(all
        .iter()
        .any(|f| f.path == "templates/scaffold/views/index.html.tera"));
    assert!(all
        .iter()
        .any(|f| f.path == "templates/controller/controller.rs.template"));

    // Filtered by generator name → only that prefix.
    let scaffold = TemplatesGenerator.generate(&["scaffold"]).unwrap();
    assert!(scaffold
        .iter()
        .all(|f| f.path.starts_with("templates/scaffold/")));
    assert!(scaffold.len() < all.len());

    // Unknown generator name errors.
    assert!(TemplatesGenerator.generate(&["bogus"]).is_err());
}

#[test]
fn test_generator_generator_scaffolds_and_registers_rust_generator() {
    let files = GeneratorGenerator.generate(&["Widget"]).unwrap();

    // A real Rust generator the app compiles in — not a runtime template folder.
    let gen = files
        .iter()
        .find(|f| f.path == "app/generators/widget.rs")
        .expect("generator source emitted");
    assert!(gen.content.contains("pub struct WidgetGenerator;"));
    assert!(gen.content.contains("impl Generator for WidgetGenerator"));
    assert!(gen.content.contains("\"widget\""));

    // Registered in the app/generators/mod.rs registry, above the marker.
    let mod_rs = files
        .iter()
        .find(|f| f.path == "app/generators/mod.rs")
        .expect("generators mod.rs emitted");
    assert!(mod_rs.content.contains("mod widget;"));
    assert!(mod_rs.content.contains("pub use widget::WidgetGenerator;"));
    assert!(mod_rs.content.contains("@generated-generators"));

    // Wired into the Doido builder in src/main.rs.
    let main_rs = files
        .iter()
        .find(|f| f.path == "src/main.rs")
        .expect("src/main.rs emitted");
    assert!(main_rs
        .content
        .contains(".register_generator(Box::new(generators::WidgetGenerator))"));
}

#[test]
fn test_project_generator_substitutes_path_and_content() {
    let root = tempdir().unwrap();
    let gen_dir = root.path().join("widget");
    fs::create_dir_all(gen_dir.join("app/{plural}")).unwrap();
    fs::write(
        gen_dir.join("app/{plural}/{snake}.rs.template"),
        "pub struct {pascal}; // {plural}\n",
    )
    .unwrap();
    fs::write(gen_dir.join("README.md"), "ignored").unwrap();

    // Listing finds the generator directory (and ignores stray files).
    fs::write(root.path().join("notes.txt"), "x").unwrap();
    assert_eq!(
        project_generator::list_in(root.path()),
        vec!["widget".to_string()]
    );

    // Discovered under the root, then processed with a name argument.
    let dir = project_generator::find_in(root.path(), "widget").unwrap();
    let files = project_generator::run(&dir, &["BlogPost"]).unwrap();

    assert_eq!(files.len(), 1); // README.md skipped
    let f = &files[0];
    assert_eq!(f.path, "app/blog_posts/blog_post.rs"); // tokens + .template stripped
    assert_eq!(f.content, "pub struct BlogPost; // blog_posts\n");
}
