pub(super) fn package_name(tool_id: &str) -> Option<&'static str> {
    match tool_id {
        "pnpm" => Some("pnpm"),
        "yarn" => Some("yarn"),
        "typescript" => Some("typescript"),
        "bun" => Some("bun"),
        _ => None,
    }
}

pub(super) fn supports_tool(tool_id: &str) -> bool {
    package_name(tool_id).is_some()
}
