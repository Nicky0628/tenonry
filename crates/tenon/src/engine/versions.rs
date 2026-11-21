use crate::backend::BackendSet;
use crate::engine::{Engine, inspect};
use crate::error::Result;
use crate::model::{BackendId, VersionList};
use crate::registry;

impl Engine {
    pub(crate) async fn list_versions(
        &self,
        tool_id: String,
        requested_backend: Option<BackendId>,
    ) -> Result<VersionList> {
        let spec = registry::find_available(&tool_id)?;
        let _permit = self.read_permit().await?;
        let backends = BackendSet::discover().await;
        let backend = backends.resolve_for_tool(spec, requested_backend)?;
        let tool = inspect::inspect_tool(spec, &backends).await?;

        if !backend.capabilities().version_selection || !tool.can_select_version {
            return Ok(VersionList {
                tool_id,
                backend: Some(backend.id()),
                versions: Vec::new(),
                message: if tool.message.is_empty() {
                    "此工具不支持指定版本。".into()
                } else {
                    tool.message
                },
            });
        }
        backend.list_versions(&self.backend_ctx, spec, &tool).await
    }
}
