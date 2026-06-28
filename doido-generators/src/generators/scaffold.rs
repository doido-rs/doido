use crate::generator::{GeneratedFile, Generator};
use crate::generators::field::Field;
use crate::generators::model::ModelGenerator;
use crate::generators::{to_pascal, to_snake, to_table_name};
use doido_core::Result;

/// Scaffold controller templates (HTML views vs JSON API), chosen by `--api`.
const CONTROLLER_HTML: &str = include_str!("../../templates/scaffold/controller_html.rs.template");
const CONTROLLER_API: &str = include_str!("../../templates/scaffold/controller_api.rs.template");

/// View templates rendered into `app/views/<plural>/` (HTML mode only).
const VIEW_INDEX: &str = include_str!("../../templates/scaffold/views/index.html.tera");
const VIEW_SHOW: &str = include_str!("../../templates/scaffold/views/show.html.tera");
const VIEW_NEW: &str = include_str!("../../templates/scaffold/views/new.html.tera");
const VIEW_EDIT: &str = include_str!("../../templates/scaffold/views/edit.html.tera");
const VIEW_FORM: &str = include_str!("../../templates/scaffold/views/_form.html.tera");

/// Fallbacks used when the app doesn't have these files on disk yet, kept in
/// sync with the generated-app templates so injection lines up.
const CONTROLLERS_MOD_BASE: &str = include_str!("../../templates/new/app/controllers/mod.rs");
const ROUTES_BASE: &str = include_str!("../../templates/new/config/routes.rs");

const CONTROLLERS_MOD_PATH: &str = "app/controllers/mod.rs";
const ROUTES_PATH: &str = "config/routes.rs";

pub struct ScaffoldGenerator;

impl Generator for ScaffoldGenerator {
    fn name(&self) -> &str {
        "scaffold"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let api = args.contains(&"--api");
        // Positional args (name + field specs); flags filtered out.
        let positional: Vec<&str> = args
            .iter()
            .copied()
            .filter(|a| !a.starts_with("--"))
            .collect();
        let name = positional.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("scaffold generator requires a name argument")
        })?;
        let fields = Field::parse_all(&positional[1..])?;

        let singular = to_snake(name); // post
        let plural = to_table_name(name); // posts
        let model = to_pascal(name); // Post
        let controller = format!("{}Controller", to_pascal(&plural)); // PostsController

        let mut files = Vec::new();

        // Model + migration + migration lib.rs + app/models/mod.rs.
        files.extend(ModelGenerator.generate(&positional)?);

        // Controller (HTML or API variant).
        let controller_template = if api { CONTROLLER_API } else { CONTROLLER_HTML };
        files.push(GeneratedFile {
            path: format!("app/controllers/{plural}_controller.rs"),
            content: render_controller(
                controller_template,
                &singular,
                &plural,
                &model,
                &controller,
                &fields,
            ),
        });

        // Register the controller module in app/controllers/mod.rs.
        let mod_existing = std::fs::read_to_string(CONTROLLERS_MOD_PATH)
            .unwrap_or_else(|_| CONTROLLERS_MOD_BASE.to_string());
        files.push(GeneratedFile {
            path: CONTROLLERS_MOD_PATH.to_string(),
            content: register_controller(&mod_existing, &plural, &controller),
        });

        // Views (HTML mode only).
        if !api {
            for (file, template) in [
                ("index", VIEW_INDEX),
                ("show", VIEW_SHOW),
                ("new", VIEW_NEW),
                ("edit", VIEW_EDIT),
                ("_form", VIEW_FORM),
            ] {
                files.push(GeneratedFile {
                    path: format!("app/views/{plural}/{file}.html.tera"),
                    content: render_view(template, &singular, &plural, &model, &fields),
                });
            }
        }

        // Inject the RESTful routes into config/routes.rs.
        let routes_existing =
            std::fs::read_to_string(ROUTES_PATH).unwrap_or_else(|_| ROUTES_BASE.to_string());
        files.push(GeneratedFile {
            path: ROUTES_PATH.to_string(),
            content: inject_route(&routes_existing, &plural, &controller),
        });

        Ok(files)
    }
}

/// Fills the controller template's field-driven fragments and names.
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
        .map(|f| format!("    {}\n", f.params_struct_field()))
        .collect();
    let active_model_sets: String = fields
        .iter()
        .map(|f| format!("            {}\n", f.active_model_set()))
        .collect();
    let active_model_assigns: String = fields
        .iter()
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

/// Fills a view template's field-driven fragments and names.
fn render_view(
    template: &str,
    singular: &str,
    plural: &str,
    model: &str,
    fields: &[Field],
) -> String {
    let table_headers: String = fields
        .iter()
        .map(|f| format!("      <th>{}</th>\n", f.column_name()))
        .collect();
    let table_cells: String = fields
        .iter()
        .map(|f| format!("      <td>{{{{ {singular}.{} }}}}</td>\n", f.column_name()))
        .collect();
    let show_fields: String = fields
        .iter()
        .map(|f| {
            let col = f.column_name();
            format!("<p><strong>{col}:</strong> {{{{ {singular}.{col} }}}}</p>\n")
        })
        .collect();
    let form_fields: String = fields.iter().map(form_field).collect();

    template
        .replace("{table_headers}", &table_headers)
        .replace("{table_cells}", &table_cells)
        .replace("{show_fields}", &show_fields)
        .replace("{form_fields}", &form_fields)
        .replace("{Model}", model)
        .replace("{singular}", singular)
        .replace("{plural}", plural)
}

/// One form control for a field, varying by HTML input type.
fn form_field(f: &Field) -> String {
    let col = f.column_name();
    match f.html_input_type() {
        "textarea" => format!("  <label>{col}<br><textarea name=\"{col}\"></textarea></label>\n"),
        "checkbox" => format!("  <label>{col} <input type=\"checkbox\" name=\"{col}\"></label>\n"),
        input => format!("  <label>{col}<br><input type=\"{input}\" name=\"{col}\"></label>\n"),
    }
}

/// Appends `mod <name>_controller;` + `pub use …` to `app/controllers/mod.rs`.
/// Idempotent: skips when the module is already declared.
fn register_controller(controllers_mod: &str, plural: &str, controller: &str) -> String {
    let module = format!("{plural}_controller");
    let decl = format!("mod {module};");
    if controllers_mod.lines().any(|l| l.trim() == decl) {
        return controllers_mod.to_string();
    }
    let mut out = controllers_mod.trim_end().to_string();
    out.push('\n');
    out.push_str(&format!("mod {module};\n"));
    out.push_str(&format!("pub use {module}::{controller};\n"));
    out
}

/// Injects `use crate::controllers::<Controller>;` and a
/// `resources!(<plural>, <Controller>);` line into `config/routes.rs`,
/// preserving existing routes. Idempotent on the resources line.
fn inject_route(routes: &str, plural: &str, controller: &str) -> String {
    let resources = format!("resources!({plural}, {controller});");
    if routes.contains(&resources) {
        return routes.to_string();
    }

    let use_line = format!("use crate::controllers::{controller};");
    let mut lines: Vec<String> = routes.lines().map(String::from).collect();

    // Add the controller import after the last existing `use crate::controllers`
    // line, or at the top otherwise.
    if !routes.contains(&use_line) {
        let pos = lines
            .iter()
            .rposition(|l| l.contains("use crate::controllers"))
            .map(|i| i + 1)
            .unwrap_or(0);
        lines.insert(pos, use_line);
    }

    // Insert the resources! call as the last statement inside `routes! { … }`.
    if let Some(open) = lines.iter().position(|l| l.contains("routes!")) {
        // Find the matching closing brace of the routes! block.
        if let Some(close_rel) = lines[open..].iter().position(|l| l.trim() == "}") {
            let close = open + close_rel;
            lines.insert(close, format!("        {resources}"));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}
