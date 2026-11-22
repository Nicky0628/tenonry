import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import { formatTimestamp, platformLabel } from '../../shared/format.js';
import { TOOL_THEMES, toolIcon, uiIcon } from '../../shared/icons.js';
import { morphCopyButton } from '../../shared/motion.js';

export function renderBackendsView(container, state, handlers) {
  const animateEntrance = Boolean(state.animateBackendsEntrance);
  state.animateBackendsEntrance = false;
  clear(container);

  const backends = state.systemReport?.backends || [];
  const tools = state.systemReport?.tools || [];
  const availableCount = backends.filter((b) => b.available).length;

  const stage = el('div', { className: 'view-stage' });
  const inner = el('div', { className: 'stage-inner' });

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
          el('h1', { className: 'view-title', text: '后端与软件源' }),
          state.systemReport
            ? el(
                'span',
                { className: 'hero-ratio' },
                el('span', { className: 'hero-ratio-num', text: String(availableCount) }),
                el('span', { className: 'hero-ratio-sep', text: '/' }),
                el('span', { className: 'hero-ratio-den', text: String(backends.length) }),
                el('span', { className: 'hero-ratio-unit', text: '可用' })
              )
            : null
        ),
        el('p', {
          className: 'view-subtitle',
          text: state.systemReport
            ? '依托本机包管理器进行来源归属、版本预检与事务协调'
            : '正在探测本机包管理器…',
        })
      ),
      el(
        'div',
        { className: 'quiet-actions' },
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-sm',
            disabled: state.loadingInventory,
            on: { click: () => handlers.refreshEnvironment() },
          },
          state.loadingInventory ? el('span', { className: 'spinner' }) : uiIcon('refresh'),
          '重新探测'
        )
      )
    )
  );

  if (state.systemReport?.issue) {
    inner.appendChild(
      el(
        'div',
        { className: 'callout is-warn' },
        el('div', { className: 'callout-title', text: '环境提示' }),
        el('div', { text: state.systemReport.issue })
      )
    );
  }

  if (backends.length === 0) {
    inner.appendChild(
      el(
        'div',
        { className: 'empty-state' },
        el('h2', {
          className: 'empty-state-title',
          text: state.loadingInventory ? '正在探测包管理器…' : '未发现支持的包管理器',
        }),
        el('p', {
          className: 'empty-state-desc',
          text: 'Tenon 依托系统及生态包管理器工作，不绕过系统包数据库。',
        })
      )
    );
  } else {
    const ledger = el('div', { className: 'ledger-list' });
    backends.forEach((backend, index) => {
      const ownedTools = tools.filter((t) => t.owner === backend.id);
      const catalogState = state.catalogStatuses.get(backend.id);
      const refreshing = state.refreshingBackends.has(backend.id);
      const caps = backend.capabilities || {};

      const leftCol = el(
        'div',
        { className: 'ledger-band-left' },
        el(
          'div',
          { className: 'ledger-title-line' },
          el('h2', { className: 'ledger-name', text: backend.name }),
          el(
            'span',
            { className: `status-inline ${backend.available ? 'is-ok' : 'is-danger'}` },
            el('span', { className: 'status-inline-dot' }),
            backend.available ? '可用' : '不可用'
          )
        ),
        el('div', {
          className: 'ledger-meta',
          text: `${caps.scope === 'ecosystem' ? '生态级' : '系统级'} · 托管 ${ownedTools.length} 项工具`,
        })
      );

      const midCol = el(
        'div',
        { className: 'ledger-band-mid' },
        el(
          'div',
          { className: 'cap-matrix' },
          capItem('指定版本', caps.versionSelection),
          capItem('批量事务合并', caps.batchTransaction),
          capItem('软件源刷新', caps.catalogRefresh),
          capItem('独立隔离索引', caps.isolatedCatalog)
        )
      );

      if (ownedTools.length > 0) {
        const strip = el('div', { className: 'owned-emblem-strip' });
        for (const tool of ownedTools) {
          const theme = TOOL_THEMES[tool.id] || 'slate';
          strip.appendChild(
            el(
              'span',
              { className: 'owned-tool-tag' },
              el('span', { className: `tool-emblem is-${theme}` }, toolIcon(tool.id)),
              el('span', { text: tool.name })
            )
          );
        }
        midCol.appendChild(strip);
      } else {
        midCol.appendChild(el('div', { className: 'ledger-meta', text: '当前无托管工具' }));
      }

      if (backend.issue) {
        midCol.appendChild(renderErrorBlock(backend.issue, { title: '探测异常' }));
      }

      if (catalogState?.error) {
        midCol.appendChild(renderErrorBlock(catalogState.error, { title: '软件源刷新失败' }));
      } else if (catalogState?.status) {
        midCol.appendChild(
          el(
            'div',
            { className: 'callout is-ok' },
            el('div', { className: 'callout-title', text: `已刷新 · ${formatTimestamp(catalogState.status.checkedAt)}` }),
            catalogState.status.message ? el('div', { text: catalogState.status.message }) : null
          )
        );
      }

      const rightCol = el('div', { className: 'ledger-band-right' });
      if (caps.catalogRefresh) {
        rightCol.appendChild(
          el(
            'button',
            {
              type: 'button',
              className: 'btn btn-sm',
              disabled: !backend.available || refreshing || Boolean(state.runningOperation),
              on: { click: () => handlers.refreshCatalog(backend.id) },
            },
            refreshing ? el('span', { className: 'spinner' }) : uiIcon('refresh'),
            refreshing ? '刷新中' : '刷新软件源'
          )
        );
      }

      const band = el(
        'section',
        { className: `ledger-band${animateEntrance ? ' is-entering' : ''}` },
        leftCol,
        midCol,
        rightCol
      );
      if (animateEntrance) band.style.setProperty('--stagger', String(index));
      ledger.appendChild(band);
    });
    inner.appendChild(ledger);
  }

  // Open environment specification sheet (no enclosing box!)
  const info = state.appInfo;
  const copyDirBtn = info?.dataDirectory
    ? el(
        'button',
        {
          type: 'button',
          className: 'btn btn-ghost btn-xs btn-copy',
          on: { click: () => void morphCopyButton(copyDirBtn, info.dataDirectory, '复制') },
        },
        uiIcon('copy'),
        '复制'
      )
    : null;

  const specSheet = el(
    'section',
    { className: 'spec-sheet' },
    el('h2', { className: 'spec-heading', text: '运行环境与本地存储' }),
    el(
      'dl',
      { className: 'kv-list' },
      el('dt', { text: '版本' }),
      el('dd', { className: 'mono', text: info ? `Tenon ${info.version}` : '—' }),
      el('dt', { text: '平台' }),
      el('dd', { text: info ? platformLabel(info.platform, info.arch) : '—' }),
      el('dt', { text: '数据目录' }),
      el(
        'dd',
        null,
        el(
          'div',
          { className: 'inline-copy-row' },
          el('span', { className: 'mono selectable', text: info?.dataDirectory || '—' }),
          copyDirBtn
        )
      ),
      el('dt', { text: '最近扫描' }),
      el('dd', { className: 'tabular', text: formatTimestamp(state.systemReport?.checkedAt) })
    )
  );

  if (info?.environmentIssue) {
    specSheet.appendChild(
      el(
        'div',
        { className: 'callout is-warn' },
        el('div', { className: 'callout-title', text: 'Shell PATH 恢复提示' }),
        el('div', { text: info.environmentIssue })
      )
    );
  }

  inner.appendChild(specSheet);
  stage.appendChild(inner);
  container.appendChild(stage);
}

function capItem(label, supported) {
  return el(
    'span',
    { className: `cap-item${supported ? '' : ' is-dim'}` },
    el('span', { className: 'cap-mark', text: supported ? '✓' : '—' }),
    el('span', { className: 'cap-label', text: label })
  );
}
