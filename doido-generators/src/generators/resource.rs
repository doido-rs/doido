//! `doido generate resource` — a model + RESTful controller + routes, without
//! views (Rails `rails generate resource`). It is the API-mode scaffold.

use crate::generator::{GeneratedFile, Generator};
use crate::generators::scaffold::ScaffoldGenerator;
use doido_core::Result;

pub struct ResourceGenerator;

impl Generator for ResourceGenerator {
    fn name(&self) -> &str {
        "resource"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        // A resource is a scaffold without views: reuse the API scaffold.
        let mut api_args: Vec<&str> = args.to_vec();
        if !api_args.contains(&"--api") {
            api_args.push("--api");
        }
        ScaffoldGenerator.generate(&api_args)
    }
}
