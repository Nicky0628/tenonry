import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import {
  CATEGORY_LABELS,
  backendLabel,
  categoryLabel,
  extractCandidateVersion,
  formatErrorSummary,
  formatShortVersion,
  formatSourceBadge,
  formatTimestamp,
} from '../../shared/format.js';
import { TOOL_THEMES, toolIcon, uiIcon } from '../../shared/icons.js';
import { animateNumber, attachSpotlight } from '../../shared/motion.js';

const FILTERS = [
  { id: 'all', label: '全部', dot: null },
  { id: 'installed', label: '已安装', dot: 'is-managed' },
  { id: 'updatable', label: '可更新', dot: 'is-updatable' },
  { id: 'attention', label: '需关注', dot: 'is-attention' },
  { id: 'missing', label: '未安装', dot: 'is-missing' },
];

let lastReadyCount = null;
let lastUnderlineGeom = null;

function isInstalledTool(tool) {
  if (tool.isProbeOnly) return Boolean(tool.found);
  return tool.state === 'installed' || tool.state === 'system' || tool.state === 'external';
}

function needsAttentionTool(tool, update) {
  if (tool.isProbeOnly) return false;
  if (tool.state === 'unavailable') return true;
  if (tool.source && (tool.source.kind === 'mixed' || tool.source.kind === 'multiBackend')) {
    return true;
  }
  if (update && update.state === 'failed') return true;
  return false;
}

function matchesFilter(tool, update, filterId) {
  switch (filterId) {
    case 'installed':
      return isInstalledTool(tool);
    case 'updatable':
      return update?.state === 'available';
    case 'attention':
      return needsAttentionTool(tool, update);
    case 'missing':
      return tool.isProbeOnly ? !tool.found : tool.state === 'missing';
    default:
      return true;
  }
}

function matchesQuery(tool, category, query) {
  if (!query) return true;
  const needle = query.toLowerCase();
  const fields = [
    tool.name,
    tool.id,
    categoryLabel(category),
    tool.version,
    tool.owner ? backendLabel(tool.owner) : '',
    ...(tool.packages || []).map((pkg) => pkg.name),
  ];
  return fields.some((value) => value && String(value).toLowerCase().includes(needle));
}

function buildUnifiedTools(state) {
  const categoryById = new Map((state.appInfo?.tools || []).map((t) => [t.id, t.category]));

  if (state.systemReport?.tools) {
    return state.systemReport.tools.map((tool) => ({
      ...tool,
      category: categoryById.get(tool.id) || 'other',
      isProbeOnly: false,
    }));
  }
  if (state.probeReport?.tools) {
    return state.probeReport.tools.map((tool) => ({
      ...tool,
      state: tool.found ? 'installed' : 'missing',
      source: null,
      owner: null,
      availableBackends: [],
      executables: [],
      packages: [],
      actions: [],
      canSelectVersion: false,
      message: tool.hint || '',
      category: categoryById.get(tool.id) || 'other',
      isProbeOnly: true,
    }));
  }
  return (state.appInfo?.tools || []).map((choice) => ({
    id: choice.id,
    name: choice.name,
    found: false,
    version: null,
    hint: null,
    state: 'missing',
    source: null,
    owner: null,
    availableBackends: [],
    executables: [],
    packages: [],
    actions: [],
    canSelectVersion: false,
    message: '',
    category: choice.category,
    isProbeOnly: true,
    isPlaceholder: true,
  }));
}

function computeMeterStats(tools, updates) {
  const denom = tools.length || 1;
  let ready = 0;
  let managedCurrent = 0;
  let updatable = 0;
  let otherInstalled = 0;
  let attention = 0;

  for (const tool of tools) {
    const update = updates.get(tool.id);
    if (isInstalledTool(tool)) ready++;

    if (needsAttentionTool(tool, update)) {
      attention++;
    } else if (update?.state === 'available') {
      updatable++;
    } else if (!tool.isProbeOnly && tool.source?.kind === 'managed') {
      managedCurrent++;
    } else if (isInstalledTool(tool)) {
      otherInstalled++;
    }
  }

  return { total: tools.length, denom, ready, managedCurrent, updatable, otherInstalled, attention };
}

function groupToolsForAllView(tools, updates) {
  const updatable = [];
  const attention = [];
  const installed = [];
  const missing = [];

  for (const tool of tools) {
    const update = updates.get(tool.id);
    if (update?.state === 'available') {
      updatable.push(tool);
    } else if (needsAttentionTool(tool, update)) {
      attention.push(tool);
    } else if (isInstalledTool(tool)) {
      installed.push(tool);
    } else {
      missing.push(tool);
    }
  }

  const groups = [];
  if (updatable.length) groups.push({ title: '可更新', items: updatable });
  if (attention.length) groups.push({ title: '需关注', items: attention });
  if (installed.length) groups.push({ title: '已就绪', items: installed });
  if (missing.length) groups.push({ title: '未安装', items: missing });
  return groups;
}

export function renderToolsView(container, state, handlers) {
  const prevScrollTop = container.querySelector('.view-stage')?.scrollTop || 0;
  const animateEntrance = Boolean(state.animateToolsEntrance);
  state.animateToolsEntrance = false;

  clear(container);

  const tools = buildUnifiedTools(state);
  const stats = computeMeterStats(tools, state.updates);
  const eligibleUpdateIds = tools
    .filter((tool) => !tool.isProbeOnly && (tool.actions || []).includes('update'))
    .map((tool) => tool.id);
  const taskLocked = Boolean(state.runningOperation || state.interruptedOperation);

  const stage = el('div', { className: 'view-stage' });
  const inner = el('div', { className: 'stage-inner' });

  // Frameless Editorial Header with inline big ratio
  const readyNumNode = el('span', { className: 'hero-ratio-num', text: String(stats.ready) });
  const subtitleText = state.loadingInventory
    ? state.probeReport
      ? '离线探测已完成，正在识别安装来源与包管理器归属…'
      : '正在离线扫描本机开发工具…'
    : state.checkingUpdates
      ? '本地清单已就绪，正在后台静默检查更新…'
      : state.systemReport?.checkedAt
        ? `上次扫描 ${formatTimestamp(state.systemReport.checkedAt)}`
        : '本机开发工具来源与版本协调';

  inner.appendChild(
    el(
      'header',
      { className: 'hero-header' },
      el(
        'div',
        null,
        el(
          'div',
          { className: 'hero-title-row' },
          el('h1', { className: 'view-title', text: '开发工具' }),
          el(
            'span',
            { className: 'hero-ratio', title: '已安装工具 / 支持工具总数' },
            readyNumNode,
            el('span', { className: 'hero-ratio-sep', text: '/' }),
            el('span', { className: 'hero-ratio-den', text: String(stats.total) }),
            el('span', { className: 'hero-ratio-unit', text: '就绪' })
          )
        ),
        el('p', { className: 'view-subtitle tabular', text: subtitleText })
      ),
      el(
        'div',
        { className: 'quiet-actions' },
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-sm',
            disabled: state.loadingInventory || state.checkingUpdates || eligibleUpdateIds.length === 0,
            on: { click: () => handlers.checkAllUpdates(true) },
          },
          state.checkingUpdates ? el('span', { className: 'spinner' }) : uiIcon('sparkles'),
          state.checkingUpdates ? '检查中' : '检查更新'
        ),
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-sm',
            disabled: state.loadingInventory,
            on: { click: () => handlers.refreshEnvironment() },
          },
          state.loadingInventory ? el('span', { className: 'spinner' }) : uiIcon('refresh'),
          state.loadingInventory ? '扫描中' : '刷新'
        )
      )
    )
  );

  animateNumber(readyNumNode, stats.ready, lastReadyCount);
  lastReadyCount = stats.ready;

  // Integrated Telemetry Proportion Rail + Unified Sliding-Underline Filter Strip
  const underline = el('div', { className: 'underline-indicator', attrs: { 'aria-hidden': 'true' } });
  const tabsTrack = el(
    'div',
    { className: 'underline-tabs', attrs: { role: 'tablist', 'aria-label': '工具状态筛选' } },
    underline
  );

  let activeTabBtn = null;
  for (const filter of FILTERS) {
    const count = tools.filter((tool) => matchesFilter(tool, state.updates.get(tool.id), filter.id)).length;
    const active = state.toolFilter === filter.id;
    const btn = el(
      'button',
      {
        type: 'button',
        className: `tab-btn${active ? ' is-active' : ''}`,
        attrs: { role: 'tab', 'aria-selected': String(active) },
        on: { click: () => handlers.setToolFilter(filter.id) },
      },
      filter.dot ? el('span', { className: `tab-dot ${filter.dot}` }) : null,
      filter.label,
      el('span', { className: 'tab-count', text: String(count) })
    );
    if (active) activeTabBtn = btn;
    tabsTrack.appendChild(btn);
  }

  const query = state.toolQuery.trim();
  const statusAndQueryTools = tools.filter((tool) => {
    const update = state.updates.get(tool.id);
    return matchesFilter(tool, update, state.toolFilter) && matchesQuery(tool, tool.category, query);
  });

  const activeCategory = state.toolCategory || 'all';
  const visibleTools =
    activeCategory === 'all'
      ? statusAndQueryTools
      : statusAndQueryTools.filter((tool) => tool.category === activeCategory);

  const presentCategories = new Set(tools.map((tool) => tool.category).filter(Boolean));
  const orderedCategories = [
    ...Object.keys(CATEGORY_LABELS).filter((id) => presentCategories.has(id)),
    ...[...presentCategories].filter((id) => !(id in CATEGORY_LABELS)),
  ];

  const categoryStrip = el('div', {
    className: 'category-strip',
    attrs: { role: 'group', 'aria-label': '工具分类筛选' },
  });
  const categoryOptions = [
    { id: 'all', label: '全部分类', count: statusAndQueryTools.length },
    ...orderedCategories.map((id) => ({
      id,
      label: categoryLabel(id),
      count: statusAndQueryTools.filter((tool) => tool.category === id).length,
    })),
  ];
  for (const option of categoryOptions) {
    const selected = activeCategory === option.id;
    categoryStrip.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: `category-chip${selected ? ' is-active' : ''}${option.count === 0 && !selected ? ' is-empty' : ''}`,
          attrs: { 'aria-pressed': String(selected) },
          on: { click: () => handlers.setToolCategory(option.id) },
        },
        el('span', { text: option.label }),
        el('span', { className: 'category-chip-count tabular', text: String(option.count) })
      )
    );
  }

  const metaRightText = query
    ? `搜索“${query}” · ${visibleTools.length} 项`
    : activeCategory !== 'all'
      ? `${categoryLabel(activeCategory)} · ${visibleTools.length} 项`
      : '';

  inner.appendChild(
    el(
      'section',
      { className: 'telemetry-strip', attrs: { 'aria-label': '环境构成与筛选' } },
      renderMeterRail(stats, handlers),
      el(
        'div',
        { className: 'tab-bar-wrap' },
        tabsTrack,
        metaRightText
          ? el('span', {
              className: 'tab-meta-right tabular',
              text: metaRightText,
            })
          : null
      ),
      orderedCategories.length > 1 ? categoryStrip : null
    )
  );

  if (state.inventoryError) {
    inner.appendChild(renderErrorBlock(state.inventoryError, { title: '环境扫描失败' }));
  }

  if (visibleTools.length === 0) {
    inner.appendChild(
      el(
        'div',
        { className: 'empty-state' },
        el('h2', {
          className: 'empty-state-title',
          text: state.loadingInventory ? '正在扫描本机环境…' : '没有匹配的工具',
        }),
        el('p', {
          className: 'empty-state-desc',
          text: state.loadingInventory
            ? '离线探测正在识别可执行文件与包管理器归属。'
            : '可以切换上方状态、分类筛选，或清空底部搜索栏关键词。',
        }),
        !state.loadingInventory &&
          (state.toolFilter !== 'all' || activeCategory !== 'all' || state.toolQuery)
          ? el(
              'button',
              {
                type: 'button',
                className: 'btn btn-sm',
                on: { click: () => handlers.resetToolFilters() },
              },
              '显示全部'
            )
          : null
      )
    );
  } else {
    const shouldGroup = state.toolFilter === 'all' && !query && !state.loadingInventory;
    const groups = shouldGroup
      ? groupToolsForAllView(visibleTools, state.updates)
      : [{ title: null, items: visibleTools }];

    let staggerCursor = 0;
    for (const group of groups) {
      const list = el('div', { className: 'row-list', attrs: { role: 'list' } });
      for (const tool of group.items) {
        const update = state.updates.get(tool.id);
        const checkingThis = state.checkingToolIds.has(tool.id);
        const isSelected = state.selectedToolId === tool.id;
        list.appendChild(
          renderToolRow(tool, update, checkingThis, isSelected, taskLocked, staggerCursor++, animateEntrance, handlers)
        );
      }

      if (group.title) {
        inner.appendChild(
          el(
            'section',
            { className: 'group-block' },
            el(
              'div',
              { className: 'group-heading' },
              el('span', { text: group.title }),
              el('span', { className: 'group-heading-count', text: String(group.items.length) })
            ),
            list
          )
        );
      } else {
        inner.appendChild(el('section', { className: 'group-block' }, list));
      }
    }
  }

  stage.appendChild(inner);
  container.appendChild(stage);
  stage.scrollTop = prevScrollTop;

  // Glide the hairline underline smoothly from previous tab to current tab
  window.requestAnimationFrame(() => {
    if (!tabsTrack.isConnected || !activeTabBtn) return;
    const trackRect = tabsTrack.getBoundingClientRect();
    const btnRect = activeTabBtn.getBoundingClientRect();
    const x = btnRect.left - trackRect.left;
    const width = btnRect.width;

    if (lastUnderlineGeom && (Math.abs(lastUnderlineGeom.x - x) > 0.5 || Math.abs(lastUnderlineGeom.width - width) > 0.5)) {
      underline.style.transition = 'none';
      underline.style.width = `${lastUnderlineGeom.width}px`;
      underline.style.transform = `translate3d(${lastUnderlineGeom.x}px, 0, 0)`;
      underline.style.opacity = '1';
      void underline.offsetWidth;
      underline.style.transition = '';
      underline.style.width = `${width}px`;
      underline.style.transform = `translate3d(${x}px, 0, 0)`;
    } else {
      underline.style.transition = 'none';
      underline.style.width = `${width}px`;
      underline.style.transform = `translate3d(${x}px, 0, 0)`;
      underline.style.opacity = '1';
      void underline.offsetWidth;
      underline.style.transition = '';
    }
    lastUnderlineGeom = { x, width };
  });
}

function renderMeterRail(stats, handlers) {
  const rail = el('div', { className: 'meter-rail', attrs: { role: 'presentation' } });
  const segments = [
    { cls: 'is-managed', count: stats.managedCurrent, filter: 'installed', title: `托管就绪 ${stats.managedCurrent}` },
    { cls: 'is-updatable', count: stats.updatable, filter: 'updatable', title: `可更新 ${stats.updatable}` },
    { cls: 'is-other', count: stats.otherInstalled, filter: 'installed', title: `附带 / 外部 ${stats.otherInstalled}` },
    { cls: 'is-attention', count: stats.attention, filter: 'attention', title: `需关注 ${stats.attention}` },
  ];
  for (const seg of segments) {
    if (seg.count <= 0) continue;
    const pct = (seg.count / stats.denom) * 100;
    const node = el('div', {
      className: `meter-segment ${seg.cls}`,
      title: seg.title,
      on: { click: () => handlers.setToolFilter(seg.filter) },
    });
    node.style.width = `${pct.toFixed(2)}%`;
    rail.appendChild(node);
  }
  return rail;
}

function renderToolRow(tool, update, checkingThis, isSelected, taskLocked, index, animateEntrance, handlers) {
  const sourceBadge = formatSourceBadge(tool);
  const candidate = update?.state === 'available' ? extractCandidateVersion(update.preview) : null;
  const actions = tool.actions || [];
  const canInstall = actions.includes('install');
  const canUpdate = actions.includes('update');
  const isMissing = !tool.isProbeOnly && tool.state === 'missing';
  const theme = TOOL_THEMES[tool.id] || 'slate';

  const identCell = el(
    'div',
    { className: 'tool-ident' },
    el('div', { className: `tool-emblem is-${theme}` }, toolIcon(tool.id)),
    el(
      'div',
      { className: 'tool-ident-text' },
      el('button', {
        type: 'button',
        className: 'tool-name-btn',
        text: tool.name,
        on: {
          click: (event) => {
            event.stopPropagation();
            handlers.openToolDetail(tool.id);
          },
        },
      }),
      el('span', { className: 'tool-category', text: categoryLabel(tool.category) })
    )
  );

  const versionCell = el('div', { className: 'tool-version-cell' });
  if (tool.version) {
    versionCell.appendChild(
      el('span', {
        className: 'version-primary mono',
        text: formatShortVersion(tool.version),
        title: tool.version,
      })
    );
  } else if (tool.found) {
    versionCell.appendChild(el('span', { className: 'version-muted', text: '已发现' }));
  } else if (tool.isPlaceholder) {
    versionCell.appendChild(el('span', { className: 'version-muted', text: '探测中…' }));
  } else {
    versionCell.appendChild(el('span', { className: 'version-muted', text: '—' }));
  }
  if (candidate) {
    versionCell.appendChild(
      el('span', {
        className: 'version-upgrade mono',
        text: `→ ${formatShortVersion(candidate)}`,
        title: `候选版本 ${candidate}`,
      })
    );
  }

  // Clean typographic status line (zero boxed badges!)
  const statusCell = el('div', { className: 'tool-status-cell' });
  const primaryTone =
    update?.state === 'available'
      ? 'is-info'
      : update?.state === 'failed'
        ? 'is-danger'
        : `is-${sourceBadge.tone}`;

  const statusLine = el(
    'div',
    { className: `status-inline ${primaryTone}` },
    el(
      'span',
      { className: 'status-indicator', attrs: { 'aria-hidden': 'true' } },
      checkingThis
        ? el('span', { className: 'spinner' })
        : el('span', { className: 'status-inline-dot' })
    ),
    el('span', { className: 'status-source', text: sourceBadge.label })
  );

  let hasStatusTail = false;
  if (tool.isProbeOnly) {
    statusLine.appendChild(el('span', { className: 'status-sep', text: '·' }));
    statusLine.appendChild(el('span', { className: 'status-tail', text: '识别归属中…' }));
    hasStatusTail = true;
  } else if (checkingThis) {
    statusLine.appendChild(el('span', { className: 'status-sep', text: '·' }));
    statusLine.appendChild(el('span', { className: 'status-tail is-info', text: '检查更新' }));
    hasStatusTail = true;
  } else if (update?.state === 'available') {
    statusLine.appendChild(el('span', { className: 'status-sep', text: '·' }));
    statusLine.appendChild(el('span', { className: 'status-tail is-info', text: '可更新' }));
    hasStatusTail = true;
  } else if (update?.state === 'current') {
    statusLine.appendChild(el('span', { className: 'status-sep', text: '·' }));
    statusLine.appendChild(el('span', { className: 'status-tail is-ok', text: '已最新' }));
    hasStatusTail = true;
  } else if (update?.state === 'failed') {
    statusLine.appendChild(el('span', { className: 'status-sep', text: '·' }));
    statusLine.appendChild(el('span', { className: 'status-tail is-danger', text: '检查失败' }));
    hasStatusTail = true;
  }
  if (hasStatusTail) {
    statusLine.classList.add('has-tail');
  }
  statusCell.appendChild(statusLine);

  const rawNote =
    update?.state === 'failed'
      ? update.message
      : !tool.isProbeOnly && tool.state !== 'installed' && tool.state !== 'missing' && tool.message
        ? tool.message
        : '';
  const noteText = rawNote
    ? update?.state === 'failed'
      ? formatErrorSummary(rawNote).summary
      : rawNote
    : '';
  if (noteText) {
    const toneClass =
      update?.state === 'failed'
        ? ' is-danger'
        : tool.source?.kind === 'mixed' || tool.source?.kind === 'multiBackend'
          ? ' is-warn'
          : '';
    statusCell.appendChild(
      el('span', { className: `tool-note${toneClass}`, text: noteText, title: noteText })
    );
  }

  // Minimal focused actions: fixed 3-slot alignment ("版本" | primary CTA | "详情")
  const actionsCell = el('div', { className: 'tool-actions-cell' });
  if (!tool.isProbeOnly) {
    if (tool.canSelectVersion) {
      actionsCell.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-sm row-quiet-action action-slot-version',
            disabled: taskLocked,
            on: {
              click: (event) => {
                event.stopPropagation();
                handlers.openVersionPicker(tool);
              },
            },
          },
          '版本'
        )
      );
    }

    if (canInstall) {
      actionsCell.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-soft btn-sm action-slot-primary',
            disabled: taskLocked,
            on: {
              click: (event) => {
                event.stopPropagation();
                handlers.openSinglePlan({
                  toolId: tool.id,
                  action: 'install',
                  backend: tool.availableBackends?.[0] || null,
                  version: null,
                });
              },
            },
          },
          uiIcon('download'),
          '安装'
        )
      );
    } else if (canUpdate && update?.state === 'available') {
      actionsCell.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-primary btn-sm action-slot-primary',
            disabled: taskLocked,
            on: {
              click: (event) => {
                event.stopPropagation();
                handlers.openSinglePlan({
                  toolId: tool.id,
                  action: 'update',
                  backend: tool.owner || null,
                  version: null,
                });
              },
            },
          },
          uiIcon('arrowUp'),
          '更新'
        )
      );
    } else if (canUpdate && update?.state === 'failed') {
      actionsCell.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-sm action-slot-primary',
            disabled: checkingThis,
            on: {
              click: (event) => {
                event.stopPropagation();
                handlers.checkSingleUpdate(tool.id);
              },
            },
          },
          uiIcon('refresh'),
          '重试'
        )
      );
    }
  }

  actionsCell.appendChild(
    el(
      'button',
      {
        type: 'button',
        className: 'btn btn-ghost btn-sm row-quiet-action action-slot-detail',
        disabled: Boolean(tool.isPlaceholder),
        on: {
          click: (event) => {
            event.stopPropagation();
            handlers.openToolDetail(tool.id);
          },
        },
      },
      '详情'
    )
  );

  const row = el(
    'div',
    {
      className: `tool-row spotlight-surface${isSelected ? ' is-selected' : ''}${isMissing ? ' is-missing' : ''}${animateEntrance ? ' is-entering' : ''}`,
      attrs: { role: 'listitem' },
      on: {
        click: () => {
          if (!tool.isPlaceholder) handlers.openToolDetail(tool.id);
        },
      },
    },
    identCell,
    versionCell,
    statusCell,
    actionsCell
  );

  if (animateEntrance) {
    row.style.setProperty('--stagger', String(Math.min(index, 12)));
  }
  attachSpotlight(row);
  return row;
}
