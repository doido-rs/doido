//! `doido generate auth:scaffold` — auth install (if needed) + auth-aware scaffold.

use super::install::AuthInstallGenerator;
use super::migration_support::{
    create_table_imports, create_table_up, register_migration, MIGRATION_LIB_BASE,
    MIGRATION_SRC_DIR,
};
use super::route_injector::{
    inject_resources, read_controllers_mod, read_models_mod, read_routes, register_controller,
    register_model_module, CONTROLLERS_MOD_PATH, MODELS_MOD_PATH, ROUTES_PATH,
};
use super::template;
use super::{to_pascal, to_snake, to_table_name, AuthGenerator, Field, GeneratedFile};
use chrono::Utc;
use doido_core::Result;

const USER_MODEL_PATH: &str = "app/models/user.rs";
const ENTITIES_MOD_PATH: &str = "app/models/_entities/mod.rs";
const ENTITIES_MOD_BASE: &str = include_str!("../../templates/new/app/models/_entities/mod.rs");

pub struct AuthScaffoldGenerator;

fn upsert_file(files: &mut Vec<GeneratedFile>, file: GeneratedFile) {
    if let Some(i) = files.iter().position(|f| f.path == file.path) {
        files[i] = file;
    } else {
        files.push(file);
    }
}

fn model_fields(fields: &[Field]) -> String {
    fields
        .iter()
        .map(|f| format!("    {}\n", f.model_field()))
        .collect()
}

fn render_controller(
    template: &str,
    singular: &str,
    plural: &str,
    model: &str,
    controller: &str,
    fields: &[Field],
) -> String {
    let params_fields: String = fields
        .iter()
        .filter(|f| !f.is_user_reference())
        .map(|f| format!("    {}\n", f.params_struct_field()))
        .collect();
    let active_model_sets: String = fields
        .iter()
        .filter(|f| !f.is_user_reference())
        .map(|f| format!("            {}\n", f.active_model_set()))
        .collect();
    let active_model_assigns: String = fields
        .iter()
        .filter(|f| !f.is_user_reference())
        .map(|f| format!("            {}\n", f.active_model_assign()))
        .collect();

    template
        .replace("{params_fields}", &params_fields)
        .replace("{active_model_sets}", &active_model_sets)
        .replace("{active_model_assigns}", &active_model_assigns)
        .replace("{Controller}", controller)
        .replace("{Model}", model)
        .replace("{singular}", singular)
        .replace("{plural}", plural)
}

fn render_view(
    template: &str,
    singular: &str,
    plural: &str,
    model: &str,
    fields: &[Field],
) -> String {
    let visible_fields: Vec<_> = fields.iter().filter(|f| !f.is_user_reference()).collect();

    let table_headers: String = visible_fields
        .iter()
        .map(|f| format!("      <th>{}</th>\n", f.column_name()))
        .collect();
    let table_cells: String = visible_fields
        .iter()
        .map(|f| format!("      <td>{{{{ {singular}.{} }}}}</td>\n", f.column_name()))
        .collect();
    let show_fields: String = visible_fields
        .iter()
        .map(|f| {
            let col = f.column_name();
            format!("<p><strong>{col}:</strong> {{{{ {singular}.{col} }}}}</p>\n")
        })
        .collect();
    let form_fields: String = visible_fields.iter().map(|f| form_field(f)).collect();

    template
        .replace("{table_headers}", &table_headers)
        .replace("{table_cells}", &table_cells)
        .replace("{show_fields}", &show_fields)
        .replace("{form_fields}", &form_fields)
        .replace("{Model}", model)
        .replace("{singular}", singular)
        .replace("{plural}", plural)
}

fn form_field(f: &Field) -> String {
    let col = f.column_name();
    match f.html_input_type() {
        "textarea" => format!("  <label>{col}<br><textarea name=\"{col}\"></textarea></label>\n"),
        "checkbox" => format!("  <label>{col} <input type=\"checkbox\" name=\"{col}\"></label>\n"),
        input => format!("  <label>{col}<br><input type=\"{input}\" name=\"{col}\"></label>\n"),
    }
}

fn ensure_user_reference(fields: &mut Vec<Field>) {
    if !fields.iter().any(|f| f.is_user_reference()) {
        fields.insert(
            0,
            Field::parse("user:references").expect("user:references is valid"),
        );
    }
}

impl AuthGenerator for AuthScaffoldGenerator {
    fn name(&self) -> &str {
        "auth:scaffold"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let api = args.contains(&"--api");
        let positional: Vec<&str> = args
            .iter()
            .copied()
            .filter(|a| !a.starts_with("--"))
            .collect();

        let name = positional.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("auth:scaffold generator requires a name argument")
        })?;

        let mut fields = Field::parse_all(&positional[1..])?;
        ensure_user_reference(&mut fields);

        let singular = to_snake(name);
        let plural = to_table_name(name);
        let model = to_pascal(name);
        let controller = format!("{}Controller", to_pascal(&plural));

        let mut files = Vec::new();

        if !std::path::Path::new(USER_MODEL_PATH).exists() {
            files.extend(AuthInstallGenerator.generate(&[])?);
        }

        let routes_base = files
            .iter()
            .find(|f| f.path == ROUTES_PATH)
            .map(|f| f.content.clone())
            .unwrap_or_else(read_routes);

        let controllers_mod_base = files
            .iter()
            .find(|f| f.path == CONTROLLERS_MOD_PATH)
            .map(|f| f.content.clone())
            .unwrap_or_else(read_controllers_mod);

        let models_mod_base = files
            .iter()
            .find(|f| f.path == MODELS_MOD_PATH)
            .map(|f| f.content.clone())
            .unwrap_or_else(read_models_mod);

        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let migration_module = format!("m{timestamp}_create_{plural}_table");
        let up_body = create_table_up(&plural, &fields);
        let migration = template("migration.rs.template")
            .replace("{migration_name}", &migration_module)
            .replace("{migration_imports}", &create_table_imports(&fields))
            .replace("{up_body}", &up_body)
            .replace(
                "{down_body}",
                &super::migration_support::drop_table_down(&plural),
            );

        let lib_path = format!("{MIGRATION_SRC_DIR}/lib.rs");
        let existing =
            std::fs::read_to_string(&lib_path).unwrap_or_else(|_| MIGRATION_LIB_BASE.to_string());
        let lib = register_migration(&existing, &migration_module);

        let entity_content = template("scaffold/entity.rs.template")
            .replace("{table_name}", &plural)
            .replace("{fields}", &model_fields(&fields));

        let extension_content = template("scaffold/model.rs.template")
            .replace("{Model}", &model)
            .replace("{singular}", &singular);

        let entities_mod_base = files
            .iter()
            .find(|f| f.path == ENTITIES_MOD_PATH)
            .map(|f| f.content.clone())
            .or_else(|| std::fs::read_to_string(ENTITIES_MOD_PATH).ok())
            .unwrap_or_else(|| ENTITIES_MOD_BASE.to_string());
        let entities_mod =
            doido_model::entities::register_entity_module(&entities_mod_base, &singular);

        let controller_template = if api {
            template("scaffold/controller_api.rs.template")
        } else {
            template("scaffold/controller_html.rs.template")
        };

        for file in [
            GeneratedFile {
                path: format!("app/models/_entities/{singular}.rs"),
                content: entity_content,
            },
            GeneratedFile {
                path: ENTITIES_MOD_PATH.to_string(),
                content: entities_mod,
            },
            GeneratedFile {
                path: format!("app/models/{singular}.rs"),
                content: extension_content,
            },
            GeneratedFile {
                path: format!("{MIGRATION_SRC_DIR}/{migration_module}.rs"),
                content: migration,
            },
            GeneratedFile {
                path: lib_path,
                content: lib,
            },
            GeneratedFile {
                path: MODELS_MOD_PATH.to_string(),
                content: register_model_module(&models_mod_base, &singular),
            },
            GeneratedFile {
                path: format!("app/controllers/{plural}_controller.rs"),
                content: render_controller(
                    controller_template,
                    &singular,
                    &plural,
                    &model,
                    &controller,
                    &fields,
                ),
            },
            GeneratedFile {
                path: CONTROLLERS_MOD_PATH.to_string(),
                content: register_controller(&controllers_mod_base, &plural, &controller),
            },
            GeneratedFile {
                path: ROUTES_PATH.to_string(),
                content: inject_resources(&routes_base, &plural, &controller, api),
            },
        ] {
            upsert_file(&mut files, file);
        }

        if !api {
            for (file, rel) in [
                ("index", "scaffold/views/index.html.tera"),
                ("show", "scaffold/views/show.html.tera"),
                ("new", "scaffold/views/new.html.tera"),
                ("edit", "scaffold/views/edit.html.tera"),
                ("_form", "scaffold/views/_form.html.tera"),
            ] {
                files.push(GeneratedFile {
                    path: format!("app/views/{plural}/{file}.html.tera"),
                    content: render_view(template(rel), &singular, &plural, &model, &fields),
                });
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_user_id_and_require_user() {
        let files = AuthScaffoldGenerator
            .generate(&["Post", "title:string"])
            .unwrap();

        let migration = files
            .iter()
            .find(|f| f.path.contains("create_posts_table"))
            .expect("posts migration");
        assert!(migration.content.contains("references(\"user\")"));

        let controller = files
            .iter()
            .find(|f| f.path.ends_with("posts_controller.rs"))
            .unwrap();
        assert!(controller.content.contains("require_user"));
        assert!(controller.content.contains("Column::UserId"));
    }
}
