use crate::generator::{GeneratedFile, Generator};
use crate::generators::{to_snake, to_pascal};
use doido_core::Result;

pub struct ChannelGenerator;

impl Generator for ChannelGenerator {
    fn name(&self) -> &str { "channel" }

    fn generate(&self, args: &[&str]) -> Result<Vec<GeneratedFile>> {
        let name = args.first().copied()
            .ok_or_else(|| doido_core::anyhow::anyhow!("channel generator requires a name argument"))?;
        let snake = to_snake(name);
        let pascal = to_pascal(name);
        Ok(vec![GeneratedFile {
            path: format!("app/channels/{}_channel.rs", snake),
            content: format!(
                "use doido_cable::{{channel, Channel, ChannelContext}};\n\n#[channel]\npub struct {pascal}Channel;\n\n#[async_trait::async_trait]\nimpl Channel for {pascal}Channel {{\n    async fn subscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {{ Ok(()) }}\n    async fn unsubscribed(&self, _ctx: &ChannelContext) -> doido_core::Result<()> {{ Ok(()) }}\n    async fn received(&self, _ctx: &ChannelContext, _data: serde_json::Value) -> doido_core::Result<()> {{ Ok(()) }}\n}}\n",
            ),
        }])
    }
}
