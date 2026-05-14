use anyhow::Result;
use crate::ui;

pub fn run() -> Result<()> {
    ui::info("8sync MCP server — phase 2");
    ui::info("Plan: expose get_project_knowledge / get_diff_image / get_project_outline");
    ui::info("Will bind to ${XDG_RUNTIME_DIR}/8sync-mcp.sock");
    Ok(())
}
