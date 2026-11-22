function tauriCore() {
  const core = window.__TAURI__?.core;
  if (!core?.invoke) {
    throw normalizeError({ kind: 'internal', message: '未检测到 Tenon 桌面运行环境。' });
  }
  return core;
}

export function normalizeError(error) {
  if (error && typeof error === 'object' && typeof error.kind === 'string' && typeof error.message === 'string') {
    return { kind: error.kind, message: error.message };
  }
  if (typeof error === 'string') {
    return { kind: 'internal', message: error };
  }
  if (error instanceof Error) {
    return { kind: 'internal', message: error.message || '发生未知错误。' };
  }
  return { kind: 'internal', message: '操作失败，请重试。' };
}

async function call(command, args = {}) {
  try {
    return await tauriCore().invoke(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

export const api = {
  appInfo: () => call('app_info'),
  setWindowTheme: (theme) => call('set_window_theme', { theme }),
  probeReport: (force = false) => call('probe_report', { force }),
  systemReport: (force = false) => call('system_report', { force }),
  selectedReport: (ids, force = false) => call('selected_report', { ids, force }),
  refreshCatalog: (backend, force = true) => call('refresh_catalog', { backend, force }),
  listVersions: (toolId, backend = null) => call('list_versions', { toolId, backend }),
  checkUpdates: (toolIds) => call('check_updates', { toolIds }),
  checkUpdate: (toolId) => call('check_update', { toolId }),
  preparePlan: (input) => call('prepare_plan', { input }),
  prepareBatch: (input) => call('prepare_batch', { input }),
  startRequest: (input) => call('start_request', { input }),
  startBatch: (input) => call('start_batch', { input }),
  operations: () => call('operations'),
  operation: (id) => call('operation', { id }),
  latestOperation: () => call('latest_operation'),
  recheckOperation: (id) => call('recheck_operation', { id }),
};

export async function listenEvent(eventName, handler) {
  const eventApi = window.__TAURI__?.event;
  if (!eventApi?.listen) return () => {};
  return eventApi.listen(eventName, (event) => handler(event.payload));
}
