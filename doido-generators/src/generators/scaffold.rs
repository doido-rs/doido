use crate::generator::{GeneratedFile, Generator};
use crate::generators::controller::{
    read_controllers_mod, register_controller_in_mod, CONTROLLERS_MOD_PATH,
};
use crate::generators::field::Field;
use crate::generators::helper::{
    helper_names, helpers_mod_path, read_helpers_mod, register_helper_in_mod, render_helper_content,
};
use crate::generators::model::ModelGenerator;
use crate::generators::{to_pascal, to_snake, to_table_name};
use doido_core::Result;

/// Fallback used when the app doesn't have `config/routes.rs` on disk yet.
const ROUTES_BASE: &str = include_str!("../../templates/new/config/routes.rs");

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
        let helper = format!("{}Helper", to_pascal(&plural)); // PostsHelper

        let mut files = Vec::new();

        // Model + migration + migration lib.rs + app/models/mod.rs.
        files.extend(ModelGenerator.generate(&positional)?);

        // Controller (HTML or API variant).
        let controller_template = if api {
            crate::templates::get("scaffold/controller_api.rs.template")
        } else {
            crate::templates::get("scaffold/controller_html.rs.template")
        };
        files.push(GeneratedFile {
            path: format!("app/controllers/{plural}_controller.rs"),
            content: render_controller(
                &controller_template,
                &singular,
                &plural,
                &model,
                &controller,
                &helper,
                &fields,
            ),
        });

        // Helper for shared controller logic.
        let (helper_snake, _) = helper_names(&plural);
        files.push(GeneratedFile {
            path: format!("app/helpers/{helper_snake}.rs"),
            content: render_helper_content(&plural, &plural),
        });
        files.push(GeneratedFile {
            path: helpers_mod_path().to_string(),
            content: register_helper_in_mod(&read_helpers_mod(), &plural),
        });

        // Register the controller module in app/controllers/mod.rs.
        files.push(GeneratedFile {
            path: CONTROLLERS_MOD_PATH.to_string(),
            content: register_controller_in_mod(&read_controllers_mod(), &plural, &controller),
        });

        // Views (HTML mode only).
        if !api {
            for (file, rel) in [
                ("index", "scaffold/views/index.html.tera"),
                ("show", "scaffold/views/show.html.tera"),
                ("new", "scaffold/views/new.html.tera"),
                ("edit", "scaffold/views/edit.html.tera"),
                ("_form", "scaffold/views/_form.html.tera"),
            ] {
                let template = crate::templates::get(rel);
                files.push(GeneratedFile {
                    path: format!("app/views/{plural}/{file}.html.tera"),
                    content: render_view(&template, &singular, &plural, &model, &fields),
                });
            }

            // Controller request tests — one per action (HTML mode only).
            let test_template = crate::templates::get("scaffold/controller_test.rs.template");
            files.push(GeneratedFile {
                path: format!("tests/{plural}_controller_test.rs"),
                content: render_controller(
                    &test_template,
                    &singular,
                    &plural,
                    &model,
                    &controller,
                    &helper,
                    &fields,
                ),
            });
        }

        // Inject the RESTful routes into config/routes.rs.
        let routes_existing =
            std::fs::read_to_string(ROUTES_PATH).unwrap_or_else(|_| ROUTES_BASE.to_string());
        files.push(GeneratedFile {
            path: ROUTES_PATH.to_string(),
            content: inject_route(&routes_existing, &plural, &controller, api),
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
    helper: &str,
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
    // Sample urlencoded body for create/update request tests.
    let form_body: String = fields
        .iter()
        .filter_map(Field::sample_form_pair)
        .collect::<Vec<_>>()
        .join("&");

    template
        .replace("{params_fields}", &params_fields)
        .replace("{active_model_sets}", &active_model_sets)
        .replace("{active_model_assigns}", &active_model_assigns)
        .replace("{form_body}", &form_body)
        .replace("{Controller}", controller)
        .replace("{Helper}", helper)
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
    let form_fields: String = fields
        .iter()
        .map(|f| f.html_form_control(singular))
        .collect();

    template
        .replace("{table_headers}", &table_headers)
        .replace("{table_cells}", &table_cells)
        .replace("{show_fields}", &show_fields)
        .replace("{form_fields}", &form_fields)
        .replace("{Model}", model)
        .replace("{singular}", singular)
        .replace("{plural}", plural)
}

/// Injects `use crate::controllers::<Controller>;` and a
/// `resources!(<plural>, <Controller>);` line into `config/routes.rs`,
/// preserving existing routes. Idempotent on the resources line.
///
/// API scaffolds omit the `new`/`edit` form actions (their controller has no
/// such methods), so the injected route excludes them — mirroring Rails, where
/// an API-only resource routes to only index/create/show/update/destroy.
fn inject_route(routes: &str, plural: &str, controller: &str, api: bool) -> String {
    let resources = if api {
        format!("resources!({plural}, {controller}, except: [new, edit]);")
    } else {
        format!("resources!({plural}, {controller});")
    };
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
