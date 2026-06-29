use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_pascal, to_snake};
use doido_core::Result;

pub struct ChannelGenerator;

impl Generator for ChannelGenerator {
    fn name(&self) -> &str {
        "channel"
    }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied().ok_or_else(|| {
            doido_core::anyhow::anyhow!("channel generator requires a name argument")
        })?;
        let snake = to_snake(name);
        let pascal = to_pascal(name);
        let content =
            crate::templates::get("channel/channel.rs.template").replace("{pascal}", &pascal);
        let test = crate::templates::get("channel/channel_test.rs.template")
            .replace("{pascal}", &pascal)
            .replace("{snake}", &snake);
        Ok(vec![
            GeneratedFile {
                path: format!("app/channels/{snake}_channel.rs"),
                content,
            },
            GeneratedFile {
                path: format!("tests/{snake}_channel_test.rs"),
                content: test,
            },
        ])
    }
}
