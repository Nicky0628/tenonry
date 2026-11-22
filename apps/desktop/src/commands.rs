use crate::{navigation::require_main_window, runtime::AppState};
use serde::Serialize;
use tauri::{State, Theme, WebviewWindow};
use tenon::{
    BackendId, BatchPlan, BatchPlanInput, BatchStartInput, CatalogStatus, Error, Operation, Plan,
    PlanInput, ProbeReport, Result, StartInput, SystemReport, ToolChoice, UpdateCheck, VersionList,
    available_tools,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppInfo {
    version: &'static str,
    platform: &'static str,
    arch: &'static str,
    data_directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment_issue: Option<String>,
    tools: Vec<ToolChoice>,
}

#[tauri::command]
pub(crate) fn app_info(window: WebviewWindow, state: State<'_, AppState>) -> Result<AppInfo> {
    require_main_window(&window)?;
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        data_directory: state.tenon.data_directory().display().to_string(),
        environment_issue: state.environment_issue.clone(),
        tools: available_tools(),
    })
}

#[tauri::command]
pub(crate) fn set_window_theme(window: WebviewWindow, theme: String) -> Result<()> {
    require_main_window(&window)?;
    let target = match theme.as_str() {
        "system" => None,
        "light" => Some(Theme::Light),
        "dark" => Some(Theme::Dark),
        _ => return Err(Error::invalid_input("无效的主题设置。")),
    };
    window
        .set_theme(target)
        .map_err(|error| Error::internal(format!("无法切换窗口主题：{error}")))
}

#[tauri::command]
pub(crate) async fn probe_report(
    window: WebviewWindow,
    state: State<'_, AppState>,
    force: bool,
) -> Result<ProbeReport> {
    require_main_window(&window)?;
    if force {
        state.tenon.refresh_probe_report().await
    } else {
        state.tenon.probe_report().await
    }
}

#[tauri::command]
pub(crate) async fn system_report(
    window: WebviewWindow,
    state: State<'_, AppState>,
    force: bool,
) -> Result<SystemReport> {
    require_main_window(&window)?;
    if force {
        state.tenon.refresh_system_report().await
    } else {
        state.tenon.system_report().await
    }
}

#[tauri::command]
pub(crate) async fn selected_report(
    window: WebviewWindow,
    state: State<'_, AppState>,
    ids: Vec<String>,
    force: bool,
) -> Result<SystemReport> {
    require_main_window(&window)?;
    state.tenon.selected_report(&ids, force).await
}

#[tauri::command]
pub(crate) async fn refresh_catalog(
    window: WebviewWindow,
    state: State<'_, AppState>,
    backend: BackendId,
    force: bool,
) -> Result<CatalogStatus> {
    require_main_window(&window)?;
    state.tenon.refresh_catalog(backend, force).await
}

#[tauri::command]
pub(crate) async fn list_versions(
    window: WebviewWindow,
    state: State<'_, AppState>,
    tool_id: String,
    backend: Option<BackendId>,
) -> Result<VersionList> {
    require_main_window(&window)?;
    state.tenon.list_versions(tool_id, backend).await
}

#[tauri::command]
pub(crate) async fn check_updates(
    window: WebviewWindow,
    state: State<'_, AppState>,
    tool_ids: Vec<String>,
) -> Result<Vec<UpdateCheck>> {
    require_main_window(&window)?;
    state.tenon.check_updates(tool_ids).await
}

#[tauri::command]
pub(crate) async fn check_update(
    window: WebviewWindow,
    state: State<'_, AppState>,
    tool_id: String,
) -> Result<UpdateCheck> {
    require_main_window(&window)?;
    state.tenon.check_update(tool_id).await
}

#[tauri::command]
pub(crate) async fn prepare_plan(
    window: WebviewWindow,
    state: State<'_, AppState>,
    input: PlanInput,
) -> Result<Plan> {
    require_main_window(&window)?;
    state.tenon.prepare_plan(input).await
}

#[tauri::command]
pub(crate) async fn prepare_batch(
    window: WebviewWindow,
    state: State<'_, AppState>,
    input: BatchPlanInput,
) -> Result<BatchPlan> {
    require_main_window(&window)?;
    state.tenon.prepare_batch(input).await
}

#[tauri::command]
pub(crate) async fn start_request(
    window: WebviewWindow,
    state: State<'_, AppState>,
    input: StartInput,
) -> Result<Operation> {
    require_main_window(&window)?;
    state.tenon.start_request(input).await
}

#[tauri::command]
pub(crate) async fn start_batch(
    window: WebviewWindow,
    state: State<'_, AppState>,
    input: BatchStartInput,
) -> Result<Operation> {
    require_main_window(&window)?;
    state.tenon.start_batch(input).await
}

#[tauri::command]
pub(crate) async fn operations(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<Operation>> {
    require_main_window(&window)?;
    let tenon = state.tenon.clone();
    tokio::task::spawn_blocking(move || tenon.operations())
        .await
        .map_err(|error| Error::internal(format!("无法读取任务记录：{error}")))?
}

#[tauri::command]
pub(crate) async fn operation(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<Operation> {
    require_main_window(&window)?;
    let tenon = state.tenon.clone();
    tokio::task::spawn_blocking(move || tenon.operation(&id))
        .await
        .map_err(|error| Error::internal(format!("无法读取任务状态：{error}")))?
}

#[tauri::command]
pub(crate) async fn latest_operation(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<Operation>> {
    require_main_window(&window)?;
    let tenon = state.tenon.clone();
    tokio::task::spawn_blocking(move || tenon.latest_operation())
        .await
        .map_err(|error| Error::internal(format!("无法读取最近任务：{error}")))?
}

#[tauri::command]
pub(crate) async fn recheck_operation(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<Operation> {
    require_main_window(&window)?;
    state.tenon.recheck_operation(&id).await
}
