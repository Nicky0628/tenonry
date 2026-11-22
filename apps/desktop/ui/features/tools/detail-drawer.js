import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import {
  backendLabel,
  categoryLabel,
  extractCandidateVersion,
  formatErrorSummary,
  formatExecutableSource,
  formatShortVersion,
  formatSourceBadge,
  formatTimestamp,
} from '../../shared/format.js';
import { TOOL_THEMES, toolIcon, uiIcon } from '../../shared/icons.js';
import { morphCopyButton } from '../../shared/motion.js';

export function renderToolDrawer(drawer, backdrop, state, handlers) {
  const toolId = state.selectedToolId;
  const tool = state.systemReport?.tools?.find((t) => t.id === toolId) || null;
  const probeTool = state.probeReport?.tools?.find((t) => t.id === toolId) || null;

  if (!toolId || (!tool && !probeTool)) {
    drawer.classList.remove('is-open');
    backdrop.classList.remove('is-open');
    drawer.setAttribute('aria-hidden', 'true');
    backdrop.setAttribute('aria-hidden', 'true');
    return;
  }

  drawer.classList.add('is-open');
  backdrop.classList.add('is-open');
  drawer.setAttribute('aria-hidden', 'false');
  backdrop.setAttribute('aria-hidden', 'false');
  clear(drawer);

  const category = state.appInfo?.tools?.find((t) => t.id === toolId)?.category || 'other';
  const update = state.updates.get(toolId);
  const checkingThis = state.checkingToolIds.has(toolId);
  const taskLocked = Boolean(state.runningOperation || state.interruptedOperation);

  const displayName = tool?.name || probeTool?.name || toolId;
  const sourceBadge = tool ? formatSourceBadge(tool) : { label: '归属识别中…', tone: 'neutral' };
  const theme = TOOL_THEMES[toolId] || 'slate';

  const header = el(
    'div',
    { className: 'drawer-header' },
    el(
      'div',
      { className: 'drawer-head-ident' },
      el('div', { className: `tool-emblem is-${theme}` }, toolIcon(toolId)),
      el(
        'div',
        null,
        el('h2', { className: 'drawer-title', text: displayName }),
        el('p', {
          className: 'view-subtitle',
          text: `${categoryLabel(category)} · ${formatShortVersion(tool?.version || probeTool?.version) || '未安装'}`,
          title: tool?.version || probeTool?.version || undefined,
        })
      )
    ),
    el(
      'button',
      {
        type: 'button',
        className: 'btn btn-ghost btn-sm',
        attrs: { 'aria-label': '关闭详情抽屉' },
        on: { click: () => handlers.closeToolDetail() },
      },
      uiIcon('close'),
      '关闭'
    )
  );

  const body = el('div', { className: 'drawer-body' });

  // Clean typographic status summary instead of stacked pill chips
  const overview = el(
    'div',
    { className: 'drawer-section' },
    el(
      'div',
      { className: `status-inline is-${sourceBadge.tone}` },
      el('span', { className: 'status-inline-dot' }),
      el('span', { text: sourceBadge.label }),
      tool?.owner ? el('span', { className: 'status-sep', text: '·' }) : null,
      tool?.owner ? el('span', { className: 'status-tail', text: `归属 ${backendLabel(tool.owner)}` }) : null,
      el('span', { className: 'status-sep', text: '·' }),
      el('span', { className: 'status-tail mono', text: toolId })
    )
  );

  if (tool?.message) {
    const tone =
      tool.state === 'unavailable'
        ? 'is-danger'
        : tool.source?.kind === 'mixed' || tool.source?.kind === 'multiBackend'
          ? 'is-warn'
          : tool.state === 'system'
            ? 'is-info'
            : '';
    overview.appendChild(
      el(
        'div',
        { className: `callout ${tone}` },
        el('div', { className: 'callout-title', text: '状态说明' }),
        el('div', { text: tool.message })
      )
    );
  } else if (!tool) {
    overview.appendChild(
      el('div', { className: 'loading-row' }, el('span', { className: 'spinner' }), '正在完成来源扫描…')
    );
  }
  body.appendChild(overview);

  // Update check section
  if (tool && (tool.actions || []).includes('update')) {
    const updateSec = el('div', { className: 'drawer-section' });
    updateSec.appendChild(
      el(
        'div',
        { className: 'record-card-head' },
        el('h3', { className: 'panel-label', text: '更新状态' }),
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-xs',
            disabled: checkingThis,
            on: { click: () => handlers.checkSingleUpdate(tool.id) },
          },
          checkingThis ? el('span', { className: 'spinner' }) : uiIcon('refresh'),
          checkingThis ? '检查中' : '重新检查'
        )
      )
    );

    if (checkingThis) {
      updateSec.appendChild(el('div', { className: 'callout is-info' }, '正在向包管理器查询候选版本…'));
    } else if (update?.state === 'available') {
      const candidate = extractCandidateVersion(update.preview);
      const basis = update.preview?.basis || '软件源索引';
      updateSec.appendChild(
        el(
          'div',
          { className: 'callout is-info' },
          el('div', {
            className: 'callout-title',
            text: candidate ? `可更新至 ${formatShortVersion(candidate)}` : '发现可用更新',
            title: candidate || undefined,
          }),
          el('div', { text: `${basis} · ${formatTimestamp(update.checkedAt)}${update.cached ? '（缓存）' : ''}` })
        )
      );
    } else if (update?.state === 'current') {
      const basis = update.preview?.basis || '软件源索引';
      updateSec.appendChild(
        el(
          'div',
          { className: 'callout is-ok' },
          el('div', { className: 'callout-title', text: '当前已是最新' }),
          el('div', { text: `${basis} · ${formatTimestamp(update.checkedAt)}${update.cached ? '（缓存）' : ''}` })
        )
      );
    } else if (update?.state === 'failed') {
      updateSec.appendChild(
        renderErrorBlock(update.message || '无法读取候选版本。', { title: '更新检查失败' })
      );
    } else {
      updateSec.appendChild(el('div', { className: 'record-path', text: '尚未检查此工具的远程更新。' }));
    }
    body.appendChild(updateSec);
  }

  // Install backend choice
  let selectedInstallBackend = tool?.availableBackends?.[0] || null;
  if (tool && (tool.actions || []).includes('install') && (tool.availableBackends || []).length > 0) {
    const installSec = el('div', { className: 'drawer-section' });
    installSec.appendChild(el('h3', { className: 'panel-label', text: '安装来源' }));
    if (tool.availableBackends.length > 1) {
      installSec.appendChild(
        el(
          'select',
          {
            className: 'backend-select',
            attrs: { 'aria-label': '选择安装后端' },
            on: { change: (event) => { selectedInstallBackend = event.target.value; } },
          },
          ...tool.availableBackends.map((backendId) =>
            el('option', { value: backendId, text: backendLabel(backendId) })
          )
        )
      );
    } else {
      installSec.appendChild(
        el('div', { className: 'record-path', text: `将通过 ${backendLabel(tool.availableBackends[0])} 安装。` })
      );
    }
    body.appendChild(installSec);
  }

  if (tool) {
    // Executables (flat hairline rows)
    const execSec = el('div', { className: 'drawer-section' });
    execSec.appendChild(
      el('h3', { className: 'panel-label', text: `可执行文件 (${(tool.executables || []).length})` })
    );
    if (!tool.executables || tool.executables.length === 0) {
      execSec.appendChild(el('div', { className: 'record-path', text: '未在 PATH 或已知位置发现可执行文件。' }));
    } else {
      const list = el('div', { className: 'record-list' });
      for (const exe of tool.executables) {
        const exeSource = formatExecutableSource(exe.source);
        const copyBtn = el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-xs btn-copy',
            on: { click: () => void morphCopyButton(copyBtn, exe.path, '复制') },
          },
          uiIcon('copy'),
          '复制'
        );
        const item = el(
          'div',
          { className: 'record-item' },
          el(
            'div',
            { className: 'record-card-head' },
            el('strong', { className: 'mono', text: exe.program }),
            el(
              'div',
              { className: 'status-inline' },
              exe.version
                ? el('span', {
                    className: 'mono record-path',
                    text: formatShortVersion(exe.version),
                    title: exe.version,
                  })
                : null,
              exe.version ? el('span', { className: 'status-sep', text: '·' }) : null,
              el('span', { className: `status-tail is-${exeSource.tone}`, text: exeSource.label }),
              exe.onPath ? el('span', { className: 'status-sep', text: '·' }) : null,
              exe.onPath ? el('span', { className: 'status-tail is-ok', text: 'PATH' }) : null
            )
          ),
          el(
            'div',
            { className: 'inline-copy-row' },
            el('span', { className: 'record-path mono selectable', text: exe.path }),
            copyBtn
          )
        );
        const showResolved =
          exe.resolvedPath &&
          (state.appInfo?.platform === 'windows'
            ? exe.resolvedPath.toLowerCase() !== exe.path.toLowerCase()
            : exe.resolvedPath !== exe.path);
        if (showResolved) {
          item.appendChild(el('div', { className: 'record-path mono selectable', text: `→ ${exe.resolvedPath}` }));
        }
        if (exe.issue) {
          const issueText = formatErrorSummary(exe.issue).summary;
          item.appendChild(el('div', { className: 'tool-note is-warn', text: issueText, title: exe.issue }));
        }
        list.appendChild(item);
      }
      execSec.appendChild(list);
    }
    body.appendChild(execSec);

    // Packages (flat hairline rows)
    const pkgSec = el('div', { className: 'drawer-section' });
    pkgSec.appendChild(
      el('h3', { className: 'panel-label', text: `包管理器记录 (${(tool.packages || []).length})` })
    );
    if (!tool.packages || tool.packages.length === 0) {
      pkgSec.appendChild(el('div', { className: 'record-path', text: '无关联软件包记录。' }));
    } else {
      const list = el('div', { className: 'record-list' });
      for (const pkg of tool.packages) {
        list.appendChild(
          el(
            'div',
            { className: 'record-item' },
            el(
              'div',
              { className: 'record-card-head' },
              el('span', { className: 'mono selectable', text: pkg.name }),
              el('span', {
                className: 'mono record-path selectable',
                text: formatShortVersion(pkg.version),
                title: pkg.version,
              })
            )
          )
        );
      }
      pkgSec.appendChild(list);
    }
    body.appendChild(pkgSec);
  }

  // Footer
  const footer = el('div', { className: 'drawer-footer' });
  const leftGroup = el('div', { className: 'drawer-footer-group' });
  const rightGroup = el('div', { className: 'drawer-footer-group' });

  if (tool) {
    const actions = tool.actions || [];
    if (actions.includes('uninstall')) {
      leftGroup.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-danger btn-sm',
            disabled: taskLocked,
            on: {
              click: () =>
                handlers.openSinglePlan({
                  toolId: tool.id,
                  action: 'uninstall',
                  backend: tool.owner || null,
                  version: null,
                }),
            },
          },
          uiIcon('trash'),
          '卸载'
        )
      );
    }

    if (tool.canSelectVersion) {
      rightGroup.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-sm',
            disabled: taskLocked,
            on: { click: () => handlers.openVersionPicker(tool, selectedInstallBackend) },
          },
          '选择版本'
        )
      );
    }

    if (actions.includes('install')) {
      rightGroup.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-primary btn-sm',
            disabled: taskLocked,
            on: {
              click: () =>
                handlers.openSinglePlan({
                  toolId: tool.id,
                  action: 'install',
                  backend: selectedInstallBackend,
                  version: null,
                }),
            },
          },
          uiIcon('download'),
          '预览安装计划'
        )
      );
    } else if (actions.includes('update')) {
      rightGroup.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: update?.state === 'available' ? 'btn btn-primary btn-sm' : 'btn btn-sm',
            disabled: taskLocked,
            on: {
              click: () =>
                handlers.openSinglePlan({
                  toolId: tool.id,
                  action: 'update',
                  backend: tool.owner || null,
                  version: null,
                }),
            },
          },
          uiIcon('arrowUp'),
          update?.state === 'available' ? '预览更新计划' : '检查并预览更新'
        )
      );
    }
  }

  footer.appendChild(leftGroup);
  footer.appendChild(rightGroup);

  drawer.appendChild(header);
  drawer.appendChild(body);
  drawer.appendChild(footer);
}
