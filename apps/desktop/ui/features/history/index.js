import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import {
  EXECUTION_LABELS,
  VERIFICATION_LABELS,
  actionLabel,
  formatDuration,
  formatErrorSummary,
  formatOperationBadge,
  formatShortVersion,
  formatTimestamp,
  phaseLabel,
} from '../../shared/format.js';
import { uiIcon } from '../../shared/icons.js';
import { morphCopyButton } from '../../shared/motion.js';

export function renderHistoryView(container, state, handlers) {
  clear(container);

  const ops = state.operations || [];
  const selected =
    ops.find((op) => op.id === state.selectedOperationId) || ops[0] || null;

  container.appendChild(
    el(
      'header',
      { className: 'history-header-bar' },
      el(
        'div',
        null,
        el(
          'div',
          { className: 'hero-title-row' },
          el('h1', { className: 'view-title', text: '任务记录' }),
          ops.length > 0
            ? el('span', { className: 'hero-ratio' }, el('span', { text: `${ops.length} 条记录` }))
            : null
        ),
        el('p', {
          className: 'view-subtitle',
          text: '每次环境变更均记录完整执行阶段、独立核验结果与命令输出',
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
            on: { click: () => handlers.refreshOperations() },
          },
          uiIcon('refresh'),
          '刷新'
        )
      )
    )
  );

  if (ops.length === 0) {
    container.appendChild(
      el(
        'div',
        { className: 'view-stage' },
        el(
          'div',
          { className: 'empty-state' },
          el('h2', { className: 'empty-state-title', text: '暂无任务记录' }),
          el('p', {
            className: 'empty-state-desc',
            text: '执行工具安装、更新、版本切换或卸载后，完整阶段与核验记录会自动保存在这里。',
          })
        )
      )
    );
    return;
  }

  const layout = el('div', { className: 'history-layout' });

  // Left vertical timeline rail
  const timelinePane = el('div', {
    className: 'timeline-pane',
    attrs: { role: 'listbox', 'aria-label': '任务时间线' },
  });
  for (const op of ops) {
    const badge = formatOperationBadge(op);
    const active = selected && selected.id === op.id;
    timelinePane.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: `timeline-item${active ? ' is-active' : ''}`,
          attrs: { role: 'option', 'aria-selected': String(Boolean(active)) },
          on: { click: () => handlers.selectOperation(op.id) },
        },
        el(
          'div',
          { className: 'timeline-rail', attrs: { 'aria-hidden': 'true' } },
          el('span', { className: `timeline-node is-${badge.tone}` }),
          el('span', { className: 'timeline-line' })
        ),
        el(
          'div',
          { className: 'timeline-content' },
          el('span', {
            className: 'timeline-title',
            text: `${actionLabel(op.action)} ${op.name}`,
          }),
          el(
            'div',
            { className: 'timeline-meta tabular' },
            el('span', { text: badge.label }),
            el('span', { text: '·' }),
            el('span', { text: formatTimestamp(op.startedAt) }),
            el('span', { text: formatDuration(op.elapsedMs) })
          )
        )
      )
    );
  }

  // Right open inspector canvas
  const inspectorPane = el('div', { className: 'inspector-pane' });
  if (selected) {
    renderInspector(inspectorPane, selected, state, handlers);
  }

  layout.appendChild(timelinePane);
  layout.appendChild(inspectorPane);
  container.appendChild(layout);
}

function renderInspector(pane, op, state, handlers) {
  const badge = formatOperationBadge(op);
  const outcome = op.outcome;
  const rechecking = state.recheckingOperationId === op.id;
  const canRecheck =
    op.state !== 'running' &&
    !state.runningOperation &&
    outcome &&
    outcome.execution !== 'notStarted' &&
    outcome.execution !== 'notNeeded';

  const actionGroup = el('div', { className: 'quiet-actions' });
  if (op.state === 'running') {
    actionGroup.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: 'btn btn-primary btn-sm',
          on: { click: () => handlers.openMonitor(op.id) },
        },
        '查看实时进度'
      )
    );
  } else if (canRecheck) {
    actionGroup.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: op.phase === 'interrupted' ? 'btn btn-primary btn-sm' : 'btn btn-sm',
          disabled: rechecking,
          on: { click: () => handlers.recheckOperation(op.id) },
        },
        rechecking ? el('span', { className: 'spinner' }) : uiIcon('shield'),
        rechecking ? '核验中…' : '重新核验状态'
      )
    );
  }

  pane.appendChild(
    el(
      'div',
      { className: 'inspector-head' },
      el(
        'div',
        null,
        el('h2', {
          className: 'inspector-title',
          text: `${actionLabel(op.action)} ${op.name}${op.version ? ` (${formatShortVersion(op.version)})` : ''}`,
          title: op.version || undefined,
        }),
        el('p', {
          className: 'inspector-sub tabular',
          text: `${formatTimestamp(op.startedAt)} · 耗时 ${formatDuration(op.elapsedMs)} · ${phaseLabel(op.phase)}`,
        })
      ),
      actionGroup
    )
  );

  if (op.message) {
    pane.appendChild(el('div', { className: 'selectable', text: formatErrorSummary(op.message).summary }));
  }

  if (op.persistenceIssue) {
    pane.appendChild(
      el(
        'div',
        { className: 'callout is-warn' },
        el('div', { className: 'callout-title', text: '日志保存异常' }),
        el('div', { text: op.persistenceIssue })
      )
    );
  }

  if (outcome) {
    const execInfo = EXECUTION_LABELS[outcome.execution] || { label: outcome.execution, tone: 'neutral' };
    const verifyInfo = VERIFICATION_LABELS[outcome.verification] || { label: outcome.verification, tone: 'neutral' };

    pane.appendChild(
      el(
        'div',
        { className: 'inspector-metrics' },
        el(
          'div',
          { className: 'metric-pair' },
          el('span', { className: 'metric-label', text: '状态' }),
          el(
            'span',
            { className: `metric-value status-inline is-${badge.tone}` },
            el('span', { className: 'status-inline-dot' }),
            badge.label
          )
        ),
        el(
          'div',
          { className: 'metric-pair' },
          el('span', { className: 'metric-label', text: '命令执行' }),
          el(
            'span',
            { className: `metric-value status-inline is-${execInfo.tone}` },
            el('span', { className: 'status-inline-dot' }),
            execInfo.label
          )
        ),
        el(
          'div',
          { className: 'metric-pair' },
          el('span', { className: 'metric-label', text: '系统核验' }),
          el(
            'span',
            { className: `metric-value status-inline is-${verifyInfo.tone}` },
            el('span', { className: 'status-inline-dot' }),
            verifyInfo.label
          )
        ),
        el(
          'div',
          { className: 'metric-pair' },
          el('span', { className: 'metric-label', text: '任务标识' }),
          el('span', { className: 'metric-value mono selectable', text: op.id.slice(0, 8) })
        )
      )
    );

    if (outcome.error) {
      pane.appendChild(renderErrorBlock(outcome.error));
    }
  }

  const items = outcome?.items?.length ? outcome.items : op.tools || [];
  if (items.length > 0) {
    const rows = el('div', { className: 'flat-rows' });
    for (const item of items) {
      const execInfo = EXECUTION_LABELS[item.execution] || { label: item.execution, tone: 'neutral' };
      const verifyInfo = VERIFICATION_LABELS[item.verification] || { label: item.verification, tone: 'neutral' };
      rows.appendChild(
        el(
          'div',
          { className: 'flat-row' },
          el(
            'div',
            { className: 'flat-row-main' },
            el('strong', { text: item.name }),
            el('span', { className: 'record-path selectable', text: item.message })
          ),
          el(
            'div',
            { className: 'flat-row-meta' },
            el(
              'span',
              { className: `status-inline is-${execInfo.tone}` },
              el('span', { className: 'status-inline-dot' }),
              execInfo.label
            ),
            el(
              'span',
              { className: `status-inline is-${verifyInfo.tone}` },
              el('span', { className: 'status-inline-dot' }),
              verifyInfo.label
            )
          )
        )
      );
    }
    pane.appendChild(
      el(
        'div',
        { className: 'panel-block' },
        el('h3', { className: 'panel-label', text: `逐项结果 (${items.length})` }),
        rows
      )
    );
  }

  // Clean terminal log viewport directly visible (or collapsible when empty)
  const logText = (op.log || outcome?.details || '').trim();
  const copyLogBtn = logText
    ? el(
        'button',
        {
          type: 'button',
          className: 'btn btn-ghost btn-xs btn-copy',
          on: { click: () => void morphCopyButton(copyLogBtn, logText, '复制日志') },
        },
        uiIcon('copy'),
        '复制日志'
      )
    : null;

  pane.appendChild(
    el(
      'div',
      { className: 'panel-block' },
      el(
        'div',
        { className: 'record-card-head' },
        el('h3', { className: 'panel-label', text: '命令输出' }),
        copyLogBtn
      ),
      el('pre', { className: 'code-block', text: logText || '此任务未产生命令输出。' })
    )
  );
}
