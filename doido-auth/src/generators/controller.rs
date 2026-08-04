//! `doido generate auth:controller` — controller with `require_user` guards.

use super::route_injector::{
    inject_action_routes, read_controllers_mod, read_routes, register_controller,
    CONTROLLERS_MOD_PATH, ROUTES_PATH,
};
use super::template;
use super::{to_pascal, to_snake, AuthGenerator, GeneratedFile};
use doido_core::Result;

pub struct AuthControllerGenerator;

fn render_action(action: &str, pascal: &str) -> String {
    match action {
        "index" => format!(
            "    /// GET /{snake}\n    pub async fn index(ctx: Context, _user: CurrentUser<User>) -> doido::Result<Response> {{\n        Ok(ctx.render(\"{snake}/index\", json!({{}})))\n    }}\n\n",
            snake = to_snake(pascal)
        ),
        "show" => format!(
            "    /// GET /{snake}/{{id}}\n    pub async fn show(ctx: Context, _user: CurrentUser<User>) -> doido::Result<Response> {{\n        let _id = parse_id(&ctx);\n        Ok(ctx.render(\"{snake}/show\", json!({{}})))\n    }}\n\n",
            snake = to_snake(pascal)
        ),
        other => format!(
            "    /// GET /{snake}/{other}\n    pub async fn {other}(ctx: Context, _user: CurrentUser<User>) -> doido::Result<Response> {{\n        Ok(ctx.json(json!({{\"action\": \"{other}\"}})))\n    }}\n\n",
            snake = to_snake(pascal)
        ),
    }
}

impl AuthGenerator for AuthControllerGenerator {
    fn name(&self) -> &str {
        "auth:controller"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let positional: Vec<&str> = args
            .iter()
            .copied()
            .filter(|a| !a.starts_with("--"))
            .collect();

        let name = positional.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("auth:controller requires a name argument")
        })?;

        let actions: Vec<&str> = if positional.len() > 1 {
            positional[1..].to_vec()
        } else {
            vec!["index"]
        };

        let snake = to_snake(name);
        let pascal = to_pascal(name);
        let controller = format!("{pascal}Controller");

        let actions_body: String = actions.iter().map(|a| render_action(a, &pascal)).collect();

        let content = template("auth_controller.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{actions}", &actions_body);

        let controllers_mod = register_controller(&read_controllers_mod(), &snake, &controller);
        let routes = inject_action_routes(&read_routes(), &snake, &controller, &actions);

        Ok(vec![
            GeneratedFile {
                path: format!("app/controllers/{snake}_controller.rs"),
                content,
            },
            GeneratedFile {
                path: CONTROLLERS_MOD_PATH.to_string(),
                content: controllers_mod,
            },
            GeneratedFile {
                path: ROUTES_PATH.to_string(),
                content: routes,
            },
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_require_user_and_routes() {
        let files = AuthControllerGenerator
            .generate(&["Dashboard", "index", "show"])
            .unwrap();
        let controller = files
            .iter()
            .find(|f| f.path == "app/controllers/dashboard_controller.rs")
            .unwrap();
        assert!(controller.content.contains("require_user"));
        assert!(controller.content.contains("CurrentUser<User>"));

        let routes = files.iter().find(|f| f.path == ROUTES_PATH).unwrap();
        assert!(routes.content.contains("DashboardController::index"));
        assert!(routes.content.contains("DashboardController::show"));
    }
}
