use clap::Args;

use crate::context::DeviceContext;

#[derive(Args)]
pub struct TuiArgs {}

pub fn handle_tui(ctx: DeviceContext) -> Result<(), String> {
    crate::tui::run(ctx).map_err(|e| e.to_string())
}
