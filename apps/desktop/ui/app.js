import { api, listenEvent, normalizeError } from './platform/api.js';
import { clear, el } from './shared/dom.js';
import { actionLabel, formatDuration, formatErrorSummary, phaseLabel } from './shared/format.js';
import { uiIcon } from './shared/icons.js';
import { syncSlidingIndicator } from './shared/motion.js';
import { renderToolsView } from './features/tools/index.js';
import { renderToolDrawer } from './features/tools/detail-drawer.js';
import { renderBackendsView } from './features/backends/index.js';
import { renderHistoryView } from './features/history/index.js';
import { renderVersionDialog } from './features/operations/version-dialog.js';
import { renderPlanDialog } from './features/operations/plan-dialog.js';
import { renderMonitorDialog } from './features/operations/monitor-dialog.js';

const THEME_STORAGE_KEY = 'tenonry.theme';

const DOCK_HINTS = [
  '搜索工具、分类、包名或后端…',
  '试试输入 node、docker、ripgrep 或 neovim…',
  '按 Ctrl+K 随时聚焦快捷检索…',
  '输入 winget、apt 或 npm 筛选归属…',
];

const state = {
  view: 'tools',
  theme: 'system',
  appInfo: null,
  probeReport: null,
  systemReport: null,
  loadingInventory: true,
  inventoryError: null,
  inventoryEpoch: 0,

  updates: new Map(),
  checkingUpdates: false,
  checkingToolIds: new Set(),
  updateEpoch: 0,

  catalogStatuses: new Map(),
  refreshingBackends: new Set(),

  toolFilter: 'all',
  toolCategory: 'all',
  toolQuery: '',
  selectedToolId: null,
  animateToolsEntrance: true,
  animateBackendsEntrance: false,

  operations: [],
  selectedOperationId: null,
  runningOperation: null,
  interruptedOperation: null,
  recheckingOperationId: null,

  modal: null,
};

let pollTimer = null;
let hintIndex = 0;
let lastTriggerElement = null;
let disposeModal = null;

const dom = {
  viewHost: document.getElementById('view-host'),
  bannerHost: document.getElementById('banner-host'),
  drawer: document.getElementById('detail-drawer'),
  drawerBackdrop: document.getElementById('drawer-backdrop'),
  modalHost: document.getElementById('modal-host'),
  toastHost: document.getElementById('toast-host'),
  navTrack: document.getElementById('nav-track'),
  navPill: document.getElementById('nav-sliding-pill'),
  navItems: [...document.querySelectorAll('.nav-item')],
  navBadgeTools: document.getElementById('nav-badge-tools'),
  navBadgeBackends: document.getElementById('nav-badge-backends'),
  navBadgeHistory: document.getElementById('nav-badge-history'),
  sidebarMeta: document.getElementById('sidebar-meta'),
  sidebarMetaText: document.getElementById('sidebar-meta-text'),
  themeSwitch: document.getElementById('theme-switch'),
  themePill: document.getElementById('theme-sliding-pill'),
  themeOptions: [...document.querySelectorAll('.theme-icon-btn')],
  dockSearchInput: document.getElementById('dock-search-input'),
  dockClearBtn: document.getElementById('dock-clear-btn'),
  dockKbd: document.getElementById('dock-kbd'),
  dockActionSlot: document.getElementById('dock-action-slot'),
};

const handlers = {
  setToolFilter(filterId) {
    if (state.toolFilter === filterId && state.view === 'tools') return;
    state.toolFilter = filterId;
    state.animateToolsEntrance = true;
    if (state.view !== 'tools') {
      state.view = 'tools';
      renderChrome();
    }
    renderCurrentView();
  },
  setToolCategory(categoryId) {
    if (state.toolCategory === categoryId && state.view === 'tools') return;
    state.toolCategory = categoryId;
    state.animateToolsEntrance = true;
    if (state.view !== 'tools') {
      state.view = 'tools';
      renderChrome();
    }
    renderCurrentView();
  },
  setToolQuery(query) {
    state.toolQuery = query;
    syncDockInput();
    if (query.trim() && state.view !== 'tools') {
      state.view = 'tools';
      renderChrome();
    }
    renderCurrentView();
  },
  resetToolFilters() {
    state.toolFilter = 'all';
    state.toolCategory = 'all';
    state.toolQuery = '';
    state.animateToolsEntrance = true;
    syncDockInput();
    renderCurrentView();
  },
  openToolDetail(toolId) {
    lastTriggerElement = document.activeElement;
    state.selectedToolId = toolId;
    renderCurrentView();
    renderDrawer();
  },
  closeToolDetail() {
    state.selectedToolId = null;
    renderCurrentView();
    renderDrawer();
    restoreFocus();
  },
  refreshEnvironment() {
    void loadEnvironment(true);
  },
  checkAllUpdates(notify = false) {
    void runBackgroundUpdateCheck(notify);
  },
  checkSingleUpdate(toolId) {
    void runSingleUpdateCheck(toolId);
  },
  refreshCatalog(backendId) {
    void runCatalogRefresh(backendId);
  },
  refreshOperations() {
    void loadOperations();
  },
  selectOperation(id) {
    state.selectedOperationId = id;
    renderCurrentView();
  },
  recheckOperation(id) {
    void runRecheckOperation(id);
  },
  openVersionPicker(tool, preferredBackend = null) {
    if (!state.modal) lastTriggerElement = document.activeElement;
    const backend = preferredBackend || tool.owner || tool.availableBackends?.[0] || null;
    state.modal = {
      type: 'version',
      tool,
      backend,
      loading: true,
      data: null,
      error: null,
      query: '',
      selectedVersion: null,
      epoch: Date.now(),
    };
    renderModal();
    void loadVersionsForModal(state.modal);
  },
  reloadVersions() {
    if (state.modal?.type !== 'version') return;
    state.modal.loading = true;
    state.modal.error = null;
    state.modal.epoch = Date.now();
    renderModal();
    void loadVersionsForModal(state.modal);
  },
  setVersionQuery(query) {
    if (state.modal?.type !== 'version') return;
    state.modal.query = query;
    state.modal.versionPage = 0;
    renderModal();
  },
  changeVersionPage(page) {
    if (state.modal?.type !== 'version') return;
    state.modal.versionPage = page;
    renderModal();
  },
  selectVersion(version) {
    if (state.modal?.type !== 'version') return;
    state.modal.selectedVersion = version;
    renderModal();
  },
  openSinglePlan(input) {
    if (!state.modal) lastTriggerElement = document.activeElement;
    state.modal = {
      type: 'plan',
      mode: 'single',
      input: {
        toolId: input.toolId,
        action: input.action,
        backend: input.backend || null,
        version: input.version || null,
      },
      loading: true,
      starting: false,
      plan: null,
      error: null,
      startError: null,
      techOpen: false,
      epoch: Date.now(),
    };
    renderModal();
    void loadPlanForModal(state.modal);
  },
  openBatchPlan(toolIds) {
    if (!toolIds || toolIds.length === 0) return;
    lastTriggerElement = document.activeElement;
    state.modal = {
      type: 'plan',
      mode: 'batch',
      candidateIds: [...toolIds],
      selectedIds: [...toolIds],
      selectionDirty: false,
      loading: true,
      starting: false,
      batchPlan: null,
      error: null,
      startError: null,
      techOpen: false,
      epoch: Date.now(),
    };
    renderModal();
    void loadPlanForModal(state.modal);
  },
  toggleBatchTool(toolId) {
    if (state.modal?.type !== 'plan' || state.modal.mode !== 'batch') return;
    const set = new Set(state.modal.selectedIds);
    if (set.has(toolId)) set.delete(toolId);
    else set.add(toolId);
    state.modal.selectedIds = state.modal.candidateIds.filter((id) => set.has(id));
    state.modal.selectionDirty = true;
    state.modal.startError = null;
    renderModal();
  },
  changeSinglePlanBackend(backendId) {
    if (state.modal?.type !== 'plan' || state.modal.mode !== 'single') return;
    state.modal.input.backend = backendId;
    state.modal.input.version = null;
    handlers.regeneratePlan();
  },
  regeneratePlan() {
    if (state.modal?.type !== 'plan') return;
    state.modal.loading = true;
    state.modal.error = null;
    state.modal.startError = null;
    state.modal.selectionDirty = false;
    state.modal.plan = null;
    state.modal.batchPlan = null;
    state.modal.epoch = Date.now();
    renderModal();
    void loadPlanForModal(state.modal);
  },
  confirmExecutePlan(boundTarget) {
    void executePlanFromModal(boundTarget);
  },
  openMonitor(operationId, retryAction = null) {
    const op =
      state.operations.find((item) => item.id === operationId) ||
      (state.runningOperation?.id === operationId ? state.runningOperation : null);
    if (!op) return;
    if (!state.modal) lastTriggerElement = document.activeElement;
    state.modal = {
      type: 'monitor',
      operationId: op.id,
      operation: op,
      logOpen: op.state === 'running',
      retryAction,
    };
    renderModal();
  },
  openHistoryFor(operationId) {
    state.modal = null;
    state.selectedToolId = null;
    state.view = 'history';
    state.selectedOperationId = operationId;
    render();
  },
  retryFromMonitor(retryAction) {
    if (!retryAction) return;
    if (retryAction.mode === 'batch') {
      handlers.openBatchPlan(retryAction.toolIds);
    } else {
      handlers.openSinglePlan(retryAction.input);
    }
  },
  closeModal() {
    if (state.modal?.starting) return;
    state.modal = null;
    renderModal();
    restoreFocus();
  },
  toast(message, tone = 'neutral') {
    showToast(message, tone);
  },
};

function restoreFocus() {
  if (lastTriggerElement && document.contains(lastTriggerElement)) {
    lastTriggerElement.focus();
  }
  lastTriggerElement = null;
}

function showToast(message, tone = 'neutral') {
  const node = el('div', { className: `toast${tone !== 'neutral' ? ` is-${tone}` : ''}`, text: message });
  dom.toastHost.appendChild(node);
  window.setTimeout(() => {
    node.remove();
  }, 3400);
}

function syncSlidingIndicators(immediate = false) {
  const activeNav = dom.navItems.find((item) => item.classList.contains('is-active'));
  syncSlidingIndicator(dom.navTrack, activeNav, dom.navPill, immediate);

  const activeTheme = dom.themeOptions.find((item) => item.classList.contains('is-active'));
  syncSlidingIndicator(dom.themeSwitch, activeTheme, dom.themePill, immediate);
}

function applyTheme(theme, persist = true, immediate = false) {
  const valid = theme === 'light' || theme === 'dark' ? theme : 'system';
  state.theme = valid;
  document.documentElement.dataset.theme = valid;
  if (persist) {
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, valid);
    } catch {
      // Ignore restricted storage errors.
    }
  }
  for (const btn of dom.themeOptions) {
    const active = btn.dataset.themeValue === valid;
    btn.classList.toggle('is-active', active);
    btn.setAttribute('aria-checked', String(active));
  }
  syncSlidingIndicators(immediate);
  void api.setWindowTheme(valid).catch(() => {});
}

function syncDockInput() {
  if (dom.dockSearchInput.value !== state.toolQuery) {
    dom.dockSearchInput.value = state.toolQuery;
  }
  const hasQuery = state.toolQuery.length > 0;
  dom.dockClearBtn.classList.toggle('is-hidden', !hasQuery);
  dom.dockKbd.classList.toggle('is-hidden', hasQuery);
}

function renderDockActionSlot() {
  clear(dom.dockActionSlot);

  if (state.runningOperation) {
    const op = state.runningOperation;
    dom.dockActionSlot.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: 'dock-island-btn is-running',
          on: { click: () => handlers.openMonitor(op.id) },
        },
        el('span', { className: 'pulse-dot' }),
        el('span', { text: `${actionLabel(op.action)} ${op.name}` }),
        el('span', { className: 'dock-island-meta', text: `${phaseLabel(op.phase)} · ${formatDuration(op.elapsedMs)}` })
      )
    );
    return;
  }

  if (state.interruptedOperation) {
    const op = state.interruptedOperation;
    const rechecking = state.recheckingOperationId === op.id;
    dom.dockActionSlot.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: 'dock-island-btn is-warn',
          disabled: rechecking,
          on: { click: () => handlers.recheckOperation(op.id) },
        },
        rechecking ? el('span', { className: 'spinner' }) : uiIcon('shield'),
        rechecking ? '核验中…' : '核验中断任务'
      )
    );
    return;
  }

  const updatableIds = [];
  for (const [toolId, check] of state.updates.entries()) {
    if (check.state === 'available') updatableIds.push(toolId);
  }
  if (updatableIds.length > 0) {
    dom.dockActionSlot.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: 'dock-island-btn',
          on: { click: () => handlers.openBatchPlan(updatableIds) },
        },
        uiIcon('arrowUp'),
        `批量更新 (${updatableIds.length})`
      )
    );
  }
}

function syncOperationDerivedState() {
  state.runningOperation = state.operations.find((op) => op.state === 'running') || null;
  state.interruptedOperation = state.operations.find((op) => op.phase === 'interrupted') || null;
  if (!state.selectedOperationId && state.operations.length > 0) {
    state.selectedOperationId = state.operations[0].id;
  }
  if (state.modal?.type === 'monitor') {
    const updated = state.operations.find((op) => op.id === state.modal.operationId);
    if (updated) {
      state.modal.operation = updated;
    }
  }
  if (state.runningOperation) {
    ensurePolling();
  } else {
    stopPolling();
  }
}

function ensurePolling() {
  if (pollTimer !== null) return;
  pollTimer = window.setInterval(() => {
    void pollRunningOperation();
  }, 450);
}

function stopPolling() {
  if (pollTimer !== null) {
    window.clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function pollRunningOperation() {
  const active = state.runningOperation;
  if (!active) {
    stopPolling();
    return;
  }
  try {
    const latest = await api.operation(active.id);
    const idx = state.operations.findIndex((op) => op.id === latest.id);
    if (idx >= 0) {
      state.operations[idx] = latest;
    } else {
      state.operations.unshift(latest);
    }
    const justFinished = latest.state !== 'running';
    syncOperationDerivedState();
    renderChrome();
    if (state.view === 'history') renderCurrentView();
    if (state.modal?.type === 'monitor' && state.modal.operationId === latest.id) {
      renderModal();
    }
    if (justFinished) {
      const doneMsg =
        latest.state === 'completed'
          ? `${actionLabel(latest.action)} ${latest.name} 已完成`
          : `${actionLabel(latest.action)} ${latest.name} 需要关注`;
      showToast(doneMsg, latest.state === 'completed' ? 'ok' : 'warn');
      void loadEnvironment(false, true);
    }
  } catch {
    // Transient polling error; retry next tick.
  }
}

async function loadOperations() {
  try {
    state.operations = await api.operations();
    syncOperationDerivedState();
    renderWorkspace();
    if (state.modal?.type === 'monitor') renderModal();
  } catch (error) {
    showToast(formatErrorSummary(normalizeError(error)).summary, 'danger');
  }
}

async function loadEnvironment(force = false, resetUpdates = force) {
  const epoch = ++state.inventoryEpoch;
  state.loadingInventory = true;
  state.inventoryError = null;
  if (resetUpdates) {
    state.updates.clear();
    state.checkingUpdates = false;
    state.checkingToolIds.clear();
    state.updateEpoch++;
  }
  renderWorkspace();

  if (!force && !state.probeReport) {
    void api
      .probeReport(false)
      .then((probe) => {
        if (epoch !== state.inventoryEpoch) return;
        state.probeReport = probe;
        state.animateToolsEntrance = true;
        renderWorkspace();
      })
      .catch(() => {});
  }

  try {
    const report = await api.systemReport(force);
    if (epoch !== state.inventoryEpoch) return;
    state.systemReport = report;
    state.loadingInventory = false;
    renderWorkspace();

    if (!state.runningOperation) {
      void runBackgroundUpdateCheck(false);
    }
  } catch (error) {
    if (epoch !== state.inventoryEpoch) return;
    state.loadingInventory = false;
    state.inventoryError = normalizeError(error);
    renderWorkspace();
  }
}

async function runBackgroundUpdateCheck(notify) {
  const tools = state.systemReport?.tools || [];
  const eligible = tools
    .filter((tool) => (tool.actions || []).includes('update'))
    .map((tool) => tool.id);
  if (eligible.length === 0) {
    state.updateEpoch++;
    state.checkingUpdates = false;
    state.checkingToolIds.clear();
    renderWorkspace();
    if (notify) showToast('当前没有可检查更新的托管工具');
    return;
  }

  const epoch = ++state.updateEpoch;
  state.checkingUpdates = true;
  state.checkingToolIds = new Set(eligible);
  renderWorkspace();

  try {
    const checks = await api.checkUpdates(eligible);
    if (epoch !== state.updateEpoch) return;
    for (const check of checks) {
      state.updates.set(check.toolId, check);
    }
    state.checkingUpdates = false;
    state.checkingToolIds.clear();
    renderWorkspace();

    if (notify) {
      const availCount = checks.filter((c) => c.state === 'available').length;
      const failCount = checks.filter((c) => c.state === 'failed').length;
      if (availCount > 0) {
        showToast(`检查完成：发现 ${availCount} 项可用更新`, 'ok');
      } else if (failCount > 0) {
        showToast(`检查完成：${failCount} 项工具更新检查失败`, 'warn');
      } else {
        showToast('检查完成：托管工具均为最新', 'ok');
      }
    }
  } catch (error) {
    if (epoch !== state.updateEpoch) return;
    state.checkingUpdates = false;
    state.checkingToolIds.clear();
    renderWorkspace();
    if (notify) showToast(formatErrorSummary(normalizeError(error)).summary, 'danger');
  }
}

async function runSingleUpdateCheck(toolId) {
  const epoch = state.updateEpoch;
  state.checkingToolIds.add(toolId);
  renderWorkspace();
  try {
    const check = await api.checkUpdate(toolId);
    if (epoch !== state.updateEpoch) return;
    state.updates.set(toolId, check);
  } catch (error) {
    if (epoch !== state.updateEpoch) return;
    const normalized = normalizeError(error);
    state.updates.set(toolId, {
      toolId,
      state: 'failed',
      backend: null,
      preview: null,
      message: normalized.message,
      checkedAt: new Date().toISOString(),
      cached: false,
    });
  } finally {
    if (epoch !== state.updateEpoch) return;
    state.checkingToolIds.delete(toolId);
    renderWorkspace();
  }
}

async function runCatalogRefresh(backendId) {
  state.refreshingBackends.add(backendId);
  renderWorkspace();
  try {
    const status = await api.refreshCatalog(backendId, true);
    state.catalogStatuses.set(backendId, { status, error: null });
    showToast(status.message || '软件源已刷新', 'ok');
    void runBackgroundUpdateCheck(false);
  } catch (error) {
    const normalized = normalizeError(error);
    state.catalogStatuses.set(backendId, { status: null, error: normalized });
    showToast(formatErrorSummary(normalized).summary, 'danger');
  } finally {
    state.refreshingBackends.delete(backendId);
    renderWorkspace();
  }
}

async function loadVersionsForModal(modalRef) {
  const epoch = modalRef.epoch;
  try {
    const data = await api.listVersions(modalRef.tool.id, modalRef.backend);
    if (state.modal !== modalRef || modalRef.epoch !== epoch) return;
    modalRef.loading = false;
    modalRef.data = data;
    const firstNonCurrent = (data.versions || []).find((item) => !item.current);
    modalRef.selectedVersion = firstNonCurrent?.version || data.versions?.[0]?.version || null;
    renderModal();
  } catch (error) {
    if (state.modal !== modalRef || modalRef.epoch !== epoch) return;
    modalRef.loading = false;
    modalRef.error = normalizeError(error);
    renderModal();
  }
}

async function loadPlanForModal(modalRef) {
  const epoch = modalRef.epoch;
  try {
    if (modalRef.mode === 'batch') {
      const batchPlan = await api.prepareBatch({ toolIds: modalRef.selectedIds });
      if (state.modal !== modalRef || modalRef.epoch !== epoch) return;
      modalRef.loading = false;
      modalRef.batchPlan = batchPlan;
    } else {
      const plan = await api.preparePlan(modalRef.input);
      if (state.modal !== modalRef || modalRef.epoch !== epoch) return;
      modalRef.loading = false;
      modalRef.plan = plan;
    }
    renderModal();
  } catch (error) {
    if (state.modal !== modalRef || modalRef.epoch !== epoch) return;
    modalRef.loading = false;
    modalRef.error = normalizeError(error);
    renderModal();
  }
}

async function executePlanFromModal(boundTarget) {
  const modalRef = state.modal;
  if (!modalRef || modalRef.type !== 'plan' || modalRef.loading || modalRef.starting) return;
  if (modalRef.selectionDirty || modalRef.error) return;
  if (boundTarget) {
    if (modalRef !== boundTarget.modal || modalRef.epoch !== boundTarget.epoch) return;
    const activeConfirmation = modalRef.mode === 'batch' ? modalRef.batchPlan : modalRef.plan;
    if (!boundTarget.confirmation || activeConfirmation !== boundTarget.confirmation) return;
  }
  const confirmation = modalRef.mode === 'batch' ? modalRef.batchPlan : modalRef.plan;
  if (!confirmation) return;

  modalRef.starting = true;
  modalRef.startError = null;
  renderModal();

  const requestId = window.crypto.randomUUID();
  const retryAction =
    modalRef.mode === 'batch'
      ? { mode: 'batch', toolIds: [...modalRef.selectedIds] }
      : { mode: 'single', input: { ...modalRef.input } };

  try {
    const op =
      modalRef.mode === 'batch'
        ? await api.startBatch({ requestId, confirmation })
        : await api.startRequest({ requestId, confirmation });

    const existingIdx = state.operations.findIndex((item) => item.id === op.id);
    if (existingIdx >= 0) {
      state.operations[existingIdx] = op;
    } else {
      state.operations.unshift(op);
    }
    state.selectedOperationId = op.id;
    syncOperationDerivedState();
    state.modal = {
      type: 'monitor',
      operationId: op.id,
      operation: op,
      logOpen: true,
      retryAction,
    };
    render();
  } catch (error) {
    if (state.modal !== modalRef) return;
    modalRef.starting = false;
    modalRef.startError = normalizeError(error);
    renderModal();
  }
}

async function runRecheckOperation(id) {
  if (state.recheckingOperationId) return;
  state.recheckingOperationId = id;
  renderWorkspace();
  if (state.modal?.type === 'monitor') renderModal();
  try {
    const updated = await api.recheckOperation(id);
    const idx = state.operations.findIndex((item) => item.id === updated.id);
    if (idx >= 0) {
      state.operations[idx] = updated;
    }
    syncOperationDerivedState();
    showToast(
      updated.state === 'completed' ? '核验完成：目标已就绪' : '核验完成：请查看逐项状态',
      updated.state === 'completed' ? 'ok' : 'warn'
    );
    void loadEnvironment(true);
  } catch (error) {
    showToast(formatErrorSummary(normalizeError(error)).summary, 'danger');
  } finally {
    state.recheckingOperationId = null;
    renderWorkspace();
    if (state.modal?.type === 'monitor') renderModal();
  }
}

function renderBanners() {
  clear(dom.bannerHost);

  if (state.interruptedOperation) {
    const op = state.interruptedOperation;
    const rechecking = state.recheckingOperationId === op.id;
    dom.bannerHost.appendChild(
      el(
        'div',
        { className: 'alert-banner is-warn' },
        el(
          'div',
          { className: 'alert-banner-text' },
          el('strong', { text: '上次任务意外中断：' }),
          el('span', {
            text: `${actionLabel(op.action)} ${op.name} 尚未确认当前状态。Tenonry 不会自动重装，核验完成后才可发起新操作。`,
          })
        ),
        el(
          'div',
          { className: 'alert-banner-actions' },
          el(
            'button',
            {
              type: 'button',
              className: 'btn btn-primary btn-xs',
              disabled: rechecking || Boolean(state.runningOperation),
              on: { click: () => handlers.recheckOperation(op.id) },
            },
            rechecking ? el('span', { className: 'spinner' }) : null,
            rechecking ? '核验中…' : '立即核验状态'
          ),
          el(
            'button',
            {
              type: 'button',
              className: 'btn btn-xs',
              on: { click: () => handlers.openHistoryFor(op.id) },
            },
            '查看记录'
          )
        )
      )
    );
  }
}

function renderChrome() {
  for (const item of dom.navItems) {
    const active = item.dataset.nav === state.view;
    item.classList.toggle('is-active', active);
    if (active) item.setAttribute('aria-current', 'page');
    else item.removeAttribute('aria-current');
  }
  syncSlidingIndicators(false);

  let updatableCount = 0;
  for (const check of state.updates.values()) {
    if (check.state === 'available') updatableCount++;
  }
  setNavBadge(dom.navBadgeTools, updatableCount > 0 ? String(updatableCount) : '', 'is-info');

  const backends = state.systemReport?.backends || [];
  const brokenBackends = backends.filter((b) => !b.available && b.issue).length;
  if (brokenBackends > 0) {
    setNavBadge(dom.navBadgeBackends, String(brokenBackends), 'is-warn');
  } else if (backends.length > 0) {
    setNavBadge(dom.navBadgeBackends, String(backends.filter((b) => b.available).length), '');
  } else {
    setNavBadge(dom.navBadgeBackends, '', '');
  }

  if (state.runningOperation) {
    setNavBadge(dom.navBadgeHistory, '运行中', 'is-running');
  } else if (state.interruptedOperation) {
    setNavBadge(dom.navBadgeHistory, '待核验', 'is-warn');
  } else if (state.operations.length > 0) {
    setNavBadge(dom.navBadgeHistory, String(state.operations.length), '');
  } else {
    setNavBadge(dom.navBadgeHistory, '', '');
  }

  // Sidebar live telemetry dot
  dom.sidebarMeta.classList.remove('is-loading', 'is-warn');
  if (state.loadingInventory) {
    dom.sidebarMeta.classList.add('is-loading');
    dom.sidebarMetaText.textContent = '离线扫描环境中…';
  } else if (state.checkingUpdates) {
    dom.sidebarMeta.classList.add('is-loading');
    dom.sidebarMetaText.textContent = '后台检查更新中…';
  } else if (state.interruptedOperation || brokenBackends > 0) {
    dom.sidebarMeta.classList.add('is-warn');
    dom.sidebarMetaText.textContent = state.interruptedOperation ? '有中断任务待核验' : `${brokenBackends} 个后端异常`;
  } else if (state.systemReport) {
    const avail = backends.filter((b) => b.available).map((b) => b.name);
    dom.sidebarMetaText.textContent = avail.length > 0 ? avail.join(' · ') : '无可用包管理器';
  } else {
    dom.sidebarMetaText.textContent = '等待就绪';
  }

  syncDockInput();
  renderDockActionSlot();
  renderBanners();
}

function setNavBadge(node, text, toneClass) {
  if (!text) {
    node.className = 'nav-count is-hidden';
    node.textContent = '';
    return;
  }
  node.className = `nav-count${toneClass ? ` ${toneClass}` : ''}`;
  node.textContent = text;
}

function renderCurrentView() {
  if (state.view === 'backends') {
    renderBackendsView(dom.viewHost, state, handlers);
  } else if (state.view === 'history') {
    renderHistoryView(dom.viewHost, state, handlers);
  } else {
    renderToolsView(dom.viewHost, state, handlers);
  }
}

function renderDrawer() {
  renderToolDrawer(dom.drawer, dom.drawerBackdrop, state, handlers);
}

function renderModal() {
  if (disposeModal) {
    const cleanup = disposeModal;
    disposeModal = null;
    cleanup();
  }
  if (!state.modal) {
    dom.modalHost.classList.remove('is-open');
    dom.modalHost.setAttribute('aria-hidden', 'true');
    clear(dom.modalHost);
    return;
  }
  dom.modalHost.classList.add('is-open');
  dom.modalHost.setAttribute('aria-hidden', 'false');
  if (state.modal.type === 'version') {
    renderVersionDialog(dom.modalHost, state.modal, handlers);
  } else if (state.modal.type === 'plan') {
    disposeModal = renderPlanDialog(dom.modalHost, state.modal, state, handlers) || null;
  } else if (state.modal.type === 'monitor') {
    renderMonitorDialog(dom.modalHost, state.modal, state, handlers);
  }
}

function renderWorkspace() {
  renderChrome();
  renderCurrentView();
  renderDrawer();
}

function render() {
  renderWorkspace();
  renderModal();
}

function wireStaticListeners() {
  for (const item of dom.navItems) {
    item.addEventListener('click', () => {
      const next = item.dataset.nav || 'tools';
      if (state.view === next) return;
      state.view = next;
      if (next === 'tools') state.animateToolsEntrance = true;
      if (next === 'backends') state.animateBackendsEntrance = true;
      renderChrome();
      renderCurrentView();
    });
  }

  for (const btn of dom.themeOptions) {
    btn.addEventListener('click', () => {
      applyTheme(btn.dataset.themeValue || 'system', true, false);
    });
  }

  dom.dockSearchInput.addEventListener('input', (event) => {
    handlers.setToolQuery(event.target.value);
  });

  dom.dockClearBtn.addEventListener('click', () => {
    handlers.setToolQuery('');
    dom.dockSearchInput.focus();
  });

  dom.drawerBackdrop.addEventListener('click', () => {
    handlers.closeToolDetail();
  });

  dom.modalHost.addEventListener('click', (event) => {
    if (event.target === dom.modalHost) {
      handlers.closeModal();
    }
  });

  window.addEventListener('resize', () => {
    syncSlidingIndicators(true);
  });

  window.addEventListener('keydown', (event) => {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
      event.preventDefault();
      dom.dockSearchInput.focus();
      dom.dockSearchInput.select();
      return;
    }
    if (
      event.key === '/' &&
      !event.ctrlKey &&
      !event.metaKey &&
      !event.altKey &&
      !['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)
    ) {
      event.preventDefault();
      dom.dockSearchInput.focus();
      return;
    }
    if (event.key !== 'Escape') return;
    if (state.modal) {
      handlers.closeModal();
    } else if (state.selectedToolId) {
      handlers.closeToolDetail();
    } else if (document.activeElement === dom.dockSearchInput) {
      if (state.toolQuery) {
        handlers.setToolQuery('');
      } else {
        dom.dockSearchInput.blur();
      }
    }
  });

  // Gently rotate idle command dock hints (DesignBookmark bottom bar style)
  window.setInterval(() => {
    if (document.activeElement === dom.dockSearchInput || state.toolQuery) return;
    hintIndex = (hintIndex + 1) % DOCK_HINTS.length;
    dom.dockSearchInput.placeholder = DOCK_HINTS[hintIndex];
  }, 4600);
}

async function bootstrap() {
  wireStaticListeners();

  let savedTheme = 'system';
  try {
    savedTheme = window.localStorage.getItem(THEME_STORAGE_KEY) || 'system';
  } catch {
    savedTheme = 'system';
  }
  applyTheme(savedTheme, false, true);

  await listenEvent('tenonry://close-blocked', (operationId) => {
    showToast('有环境任务正在执行，完成后再关闭窗口。', 'warn');
    if (operationId) {
      handlers.openMonitor(operationId);
    }
  });

  render();
  window.requestAnimationFrame(() => syncSlidingIndicators(true));

  const [infoResult, opsResult] = await Promise.allSettled([
    api.appInfo(),
    api.operations(),
  ]);

  if (infoResult.status === 'fulfilled') {
    state.appInfo = infoResult.value;
  } else {
    state.inventoryError = normalizeError(infoResult.reason);
  }

  if (opsResult.status === 'fulfilled') {
    state.operations = opsResult.value;
    syncOperationDerivedState();
  }

  render();
  await loadEnvironment(false);
}

void bootstrap();
