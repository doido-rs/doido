use crate::generator::{GeneratedFile, Generator};
use crate::generators::{register_module, to_pascal, to_snake};
use doido_core::Result;

/// Fallback `app/channels/mod.rs` when the app doesn't have one on disk yet.
const CHANNELS_MOD_BASE: &str = include_str!("../../templates/new/app/channels/mod.rs");
const CHANNELS_MOD_PATH: &str = "app/channels/mod.rs";

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

        // Register the channel's module in app/channels/mod.rs.
        let existing = std::fs::read_to_string(CHANNELS_MOD_PATH)
            .unwrap_or_else(|_| CHANNELS_MOD_BASE.to_string());
        let channels_mod = register_module(
            &existing,
            &format!("{snake}_channel"),
            "@generated-channels",
        );

        Ok(vec![
            GeneratedFile {
                path: format!("app/channels/{snake}_channel.rs"),
                content,
            },
            GeneratedFile {
                path: CHANNELS_MOD_PATH.to_string(),
                content: channels_mod,
            },
            GeneratedFile {
                path: format!("tests/{snake}_channel_test.rs"),
                content: test,
            },
        ])
    }
}
