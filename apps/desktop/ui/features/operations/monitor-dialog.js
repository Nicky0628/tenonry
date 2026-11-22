import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import {
  EXECUTION_LABELS,
  VERIFICATION_LABELS,
  actionLabel,
  formatDuration,
  formatErrorSummary,
  formatOperationBadge,
  formatShortVersion,
  phaseLabel,
} from '../../shared/format.js';
import { uiIcon } from '../../shared/icons.js';
import { morphCopyButton } from '../../shared/motion.js';

const ORDERED_STAGES = [
  { key: 'checking', label: '复核计划', phases: ['checking'] },
  { key: 'authorizing', label: '系统授权', phases: ['authorizing'] },
  {
    key: 'executing',
    label: '执行变更',
    phases: ['downloading', 'installing', 'configuring', 'removing', 'executing'],
  },
  { key: 'verifying', label: '核验结果', phases: ['verifying'] },
  { key: 'finished', label: '完成', phases: ['finished'] },
];

function stageIndexFor(phase) {
  if (phase === 'interrupted') return 2;
  const idx = ORDERED_STAGES.findIndex((stage) => stage.phases.includes(phase));
  return idx >= 0 ? idx : 0;
}

export function renderMonitorDialog(host, modal, state, handlers) {
  const prevLog = host.querySelector('.monitor-log-block');
  const wasAtBottom =
    prevLog ? prevLog.scrollHeight - prevLog.scrollTop - prevLog.clientHeight < 24 : true;

  clear(host);

  const op = modal.operation;
  if (!op) return;

  const isRunning = op.state === 'running';
  const badge = formatOperationBadge(op);
  const outcome = op.outcome;
  const rechecking = state.recheckingOperationId === op.id;

  const dialog = el('div', {
    className: 'modal-dialog',
    attrs: { role: 'dialog', 'aria-modal': 'true', 'aria-label': '任务执行进度' },
  });

  dialog.appendChild(
    el(
      'div',
      { className: 'modal-header' },
      el(
        'div',
        null,
        el('h2', {
          className: 'modal-title',
          text: `${actionLabel(op.action)} ${op.name}${op.version ? ` (${formatShortVersion(op.version)})` : ''}`,
          title: op.version || undefined,
        }),
        el('p', {
          className: 'modal-subtitle tabular',
          text: isRunning
            ? `已用时 ${formatDuration(op.elapsedMs)} · 关闭此窗口不会中止后台执行`
            : `总耗时 ${formatDuration(op.elapsedMs)} · ${phaseLabel(op.phase)}`,
        })
      ),
      el(
        'div',
        { className: 'quiet-actions' },
        el(
          'span',
          { className: `status-inline is-${badge.tone}` },
          isRunning ? el('span', { className: 'spinner' }) : el('span', { className: 'status-inline-dot' }),
          badge.label
        ),
        el(
          'button',
          {
            type: 'button',
            className: 'btn btn-ghost btn-sm',
            on: { click: () => handlers.closeModal() },
          },
          isRunning ? '转入后台' : '关闭'
        )
      )
    )
  );

  const body = el('div', { className: 'modal-body' });

  const activeIdx = stageIndexFor(op.phase);
  const stepper = el('div', { className: 'phase-strip', attrs: { 'aria-label': '执行阶段' } });
  ORDERED_STAGES.forEach((stage, index) => {
    let cls = 'phase-step';
    if (op.phase === 'interrupted' && index === activeIdx) {
      cls += ' is-warn';
    } else if (op.state === 'completed' || index < activeIdx) {
      cls += ' is-done';
    } else if (index === activeIdx) {
      cls += op.state === 'needsAttention' ? ' is-warn' : ' is-active';
    }
    const label = index === 2 && stage.phases.includes(op.phase) ? phaseLabel(op.phase) : stage.label;
    stepper.appendChild(el('span', { className: cls, text: label }));
  });
  body.appendChild(stepper);

  const messageTone =
    op.state === 'completed'
      ? 'is-ok'
      : op.state === 'needsAttention'
        ? 'is-warn'
        : 'is-info';
  body.appendChild(
    el(
      'div',
      { className: `callout ${messageTone}` },
      el('div', { className: 'callout-title', text: phaseLabel(op.phase) }),
      el('div', { className: 'selectable', text: op.message ? formatErrorSummary(op.message).summary : '正在处理…' })
    )
  );

  if (op.persistenceIssue) {
    body.appendChild(
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

    body.appendChild(
      el(
        'div',
        { className: 'summary-strip' },
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
        )
      )
    );

    if (outcome.error) {
      body.appendChild(renderErrorBlock(outcome.error));
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
    body.appendChild(
      el(
        'div',
        { className: 'panel-block' },
        el('h3', { className: 'panel-label', text: `任务项目 (${items.length})` }),
        rows
      )
    );
  }

  const logText = (op.log || outcome?.details || '').trim();
  const logPre = el('pre', {
    className: 'code-block monitor-log-block',
    text: logText || (isRunning ? '等待命令输出…' : '无命令输出。'),
  });
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

  body.appendChild(
    el(
      'details',
      {
        className: 'tech-details',
        open: Boolean(modal.logOpen),
        on: {
          toggle: (event) => {
            modal.logOpen = event.target.open;
          },
        },
      },
      el('summary', { text: isRunning ? '实时命令输出' : '完整命令输出' }),
      el(
        'div',
        { className: 'tech-details-body' },
        copyLogBtn ? el('div', { className: 'inline-copy-row' }, copyLogBtn) : null,
        logPre
      )
    )
  );

  const footer = el('div', { className: 'modal-footer modal-footer-split' });
  const left = el('div', { className: 'drawer-footer-group' });
  const right = el('div', { className: 'drawer-footer-group' });

  const canRecheck =
    !isRunning &&
    outcome &&
    outcome.execution !== 'notStarted' &&
    outcome.execution !== 'notNeeded';

  if (canRecheck && op.state === 'needsAttention') {
    left.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: 'btn btn-primary',
          disabled: rechecking,
          on: { click: () => handlers.recheckOperation(op.id) },
        },
        rechecking ? el('span', { className: 'spinner' }) : uiIcon('shield'),
        rechecking ? '核验中…' : '重新核验当前状态'
      )
    );
  }

  if (!isRunning && outcome?.error?.kind === 'planExpired' && modal.retryAction) {
    left.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: 'btn btn-primary',
          on: { click: () => handlers.retryFromMonitor(modal.retryAction) },
        },
        uiIcon('refresh'),
        '重新生成计划'
      )
    );
  }

  right.appendChild(
    el(
      'button',
      {
        type: 'button',
        className: 'btn btn-ghost',
        on: { click: () => handlers.openHistoryFor(op.id) },
      },
      '在任务记录中查看'
    )
  );

  right.appendChild(
    el(
      'button',
      {
        type: 'button',
        className: isRunning ? 'btn' : 'btn btn-primary',
        on: { click: () => handlers.closeModal() },
      },
      isRunning ? '转入后台' : '完成'
    )
  );

  footer.appendChild(left);
  footer.appendChild(right);

  dialog.appendChild(body);
  dialog.appendChild(footer);
  host.appendChild(dialog);

  if (modal.logOpen && wasAtBottom) {
    logPre.scrollTop = logPre.scrollHeight;
  }
}
