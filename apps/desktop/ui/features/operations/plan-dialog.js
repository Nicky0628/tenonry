import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import {
  actionLabel,
  backendLabel,
  formatCommandLine,
  formatShortVersion,
} from '../../shared/format.js';
import { uiIcon } from '../../shared/icons.js';
import { attachHoldToConfirm } from '../../shared/motion.js';

export function renderPlanDialog(host, modal, state, handlers) {
  clear(host);

  const isBatch = modal.mode === 'batch';
  const dialog = el('div', {
    className: 'modal-dialog',
    attrs: {
      role: 'dialog',
      'aria-modal': 'true',
      'aria-label': isBatch ? '审阅批量更新计划' : '审阅操作计划',
    },
  });

  const titleText = isBatch
    ? '审阅批量更新计划'
    : `${actionLabel(modal.input.action)} ${resolveToolName(modal, state)}${modal.input.version ? ` (${formatShortVersion(modal.input.version)})` : ''}`;

  const subtitleText = isBatch
    ? '核对各包管理器事务与版本变更后再执行。'
    : '执行前已通过包管理器预检，请确认目标版本与影响范围。';

  dialog.appendChild(
    el(
      'div',
      { className: 'modal-header' },
      el(
        'div',
        null,
        el('h2', { className: 'modal-title', text: titleText }),
        el('p', { className: 'modal-subtitle', text: subtitleText })
      ),
      el(
        'button',
        {
          type: 'button',
          className: 'btn btn-ghost btn-sm',
          disabled: modal.starting,
          on: { click: () => handlers.closeModal() },
        },
        uiIcon('close'),
        '关闭'
      )
    )
  );

  const body = el('div', { className: 'modal-body' });

  if (isBatch && (modal.candidateIds || []).length > 1) {
    const pickerBlock = el('div', { className: 'panel-block' });
    pickerBlock.appendChild(
      el('h3', {
        className: 'panel-label',
        text: `包含工具 (${modal.selectedIds.length}/${modal.candidateIds.length})`,
      })
    );
    const list = el('div', { className: 'record-list' });
    for (const id of modal.candidateIds) {
      const tool = state.systemReport?.tools?.find((t) => t.id === id);
      const checked = modal.selectedIds.includes(id);
      list.appendChild(
        el(
          'label',
          { className: 'checkbox-row' },
          el('input', {
            type: 'checkbox',
            checked,
            disabled: modal.loading || modal.starting,
            on: { change: () => handlers.toggleBatchTool(id) },
          }),
          el('strong', { className: 'checkbox-label', text: tool?.name || id }),
          tool?.owner
            ? el('span', { className: 'record-path checkbox-meta', text: backendLabel(tool.owner) })
            : null
        )
      );
    }
    pickerBlock.appendChild(list);
    if (modal.selectionDirty) {
      pickerBlock.appendChild(
        el(
          'div',
          { className: 'callout is-info' },
          el('div', { text: '已调整批量更新范围，请重新生成计划以核验最新事务。' })
        )
      );
    }
    body.appendChild(pickerBlock);
  }

  if (!isBatch && modal.input.action === 'install') {
    const tool = state.systemReport?.tools?.find((t) => t.id === modal.input.toolId);
    if (tool && (tool.availableBackends || []).length > 1) {
      body.appendChild(
        el(
          'div',
          { className: 'inline-copy-row' },
          el('span', { className: 'panel-label', text: '目标后端' }),
          el(
            'select',
            {
              className: 'backend-select',
              disabled: modal.loading || modal.starting,
              attrs: { 'aria-label': '切换安装后端' },
              on: { change: (event) => handlers.changeSinglePlanBackend(event.target.value) },
            },
            ...tool.availableBackends.map((backendId) =>
              el('option', {
                value: backendId,
                text: backendLabel(backendId),
                attrs: backendId === modal.input.backend ? { selected: 'selected' } : null,
              })
            )
          )
        )
      );
    }
  }

  if (modal.loading) {
    body.appendChild(
      el(
        'div',
        { className: 'loading-row' },
        el('span', { className: 'spinner' }),
        '正在向包管理器核验版本与依赖变更，这不会修改本机软件…'
      )
    );
  } else if (modal.error) {
    body.appendChild(renderErrorBlock(modal.error));
  } else if (isBatch && modal.batchPlan) {
    renderBatchPlanBody(body, modal.batchPlan, modal);
  } else if (!isBatch && modal.plan) {
    renderSinglePlanBody(body, modal.plan, modal);
  }

  if (modal.startError) {
    body.appendChild(renderErrorBlock(modal.startError));
  }

  const footer = el('div', { className: 'modal-footer modal-footer-split' });
  const left = el('div', { className: 'drawer-footer-group' });
  const right = el('div', { className: 'drawer-footer-group' });

  const isExpired = modal.startError?.kind === 'planExpired' || modal.error?.kind === 'planExpired';
  const hasExecutableCommands = isBatch
    ? Boolean(modal.batchPlan?.transactions?.some((tx) => (tx.commands || []).length > 0))
    : Boolean(modal.plan && modal.plan.preview?.state !== 'current' && (modal.plan.commands || []).length > 0);

  if (modal.error || modal.startError || modal.selectionDirty) {
    left.appendChild(
      el(
        'button',
        {
          type: 'button',
          className: isExpired || modal.selectionDirty ? 'btn btn-primary' : 'btn',
          disabled: modal.loading || modal.starting || (isBatch && modal.selectedIds.length === 0),
          on: { click: () => handlers.regeneratePlan() },
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
        disabled: modal.starting,
        on: { click: () => handlers.closeModal() },
      },
      '取消'
    )
  );

  let cleanupHold = null;

  if (!modal.loading && !modal.error && !modal.selectionDirty) {
    const isUninstall = !isBatch && modal.input.action === 'uninstall';
    const disabled = modal.starting || !hasExecutableCommands || isExpired;
    const boundTarget = {
      modal,
      epoch: modal.epoch,
      confirmation: isBatch ? modal.batchPlan : modal.plan,
    };

    if (isUninstall && !disabled) {
      const holdBtn = el(
        'button',
        {
          type: 'button',
          className: 'btn btn-hold',
          title: '长按以确认卸载，防止误触',
        },
        uiIcon('trash'),
        el('span', { text: '按住确认卸载', attrs: { 'data-hold-label': '' } })
      );
      cleanupHold = attachHoldToConfirm(holdBtn, {
        durationMs: 780,
        onConfirm: () => handlers.confirmExecutePlan(boundTarget),
      });
      right.appendChild(holdBtn);
    } else {
      const confirmLabel = modal.starting
        ? '正在启动…'
        : !hasExecutableCommands
          ? '无需执行'
          : isBatch
            ? `确认批量更新 (${modal.batchPlan.plans.length})`
            : `确认${actionLabel(modal.input.action)}`;

      right.appendChild(
        el(
          'button',
          {
            type: 'button',
            className: isUninstall ? 'btn btn-danger' : 'btn btn-primary',
            disabled,
            on: { click: () => handlers.confirmExecutePlan(boundTarget) },
          },
          modal.starting ? el('span', { className: 'spinner' }) : null,
          confirmLabel
        )
      );
    }
  }

  footer.appendChild(left);
  footer.appendChild(right);

  dialog.appendChild(body);
  dialog.appendChild(footer);
  host.appendChild(dialog);

  return () => {
    if (cleanupHold) {
      cleanupHold();
      cleanupHold = null;
    }
  };
}

function resolveToolName(modal, state) {
  if (modal.plan?.tool?.name) return modal.plan.tool.name;
  const found = state.systemReport?.tools?.find((t) => t.id === modal.input.toolId);
  return found?.name || modal.input.toolId;
}

function renderSinglePlanBody(body, plan, modal) {
  const needsAdmin = (plan.commands || []).some((cmd) => cmd.administrator);
  const isCurrent = plan.preview?.state === 'current' || (plan.commands || []).length === 0;

  body.appendChild(
    el(
      'div',
      { className: 'summary-strip' },
      metricPair('操作', actionLabel(plan.action)),
      metricPair('后端', backendLabel(plan.backend)),
      metricPair('系统权限', isCurrent ? '无需提权' : needsAdmin ? '需要管理员授权' : '当前用户权限')
    )
  );

  if (isCurrent) {
    body.appendChild(
      el(
        'div',
        { className: 'callout is-ok' },
        el('div', { className: 'callout-title', text: '当前已是最新状态' }),
        el('div', { text: plan.impact || '没有需要应用的软件包变更，不会触发安装或提权。' })
      )
    );
  } else if (plan.action === 'uninstall') {
    body.appendChild(
      el(
        'div',
        { className: 'callout is-warn' },
        el('div', { className: 'callout-title', text: '卸载确认（长按下方按钮执行）' }),
        el('div', { text: plan.impact })
      )
    );
  } else if (plan.impact) {
    body.appendChild(el('div', { className: 'selectable', text: plan.impact }));
  }

  const targets = plan.preview?.targets || [];
  if (targets.length > 0) {
    body.appendChild(renderChangeList('目标软件包变更', targets));
  }

  body.appendChild(renderTechDetails(plan.preview, plan.commands, modal));
}

function renderBatchPlanBody(body, batch, modal) {
  const txCount = (batch.transactions || []).length;
  const totalTools = (batch.plans || []).length;
  const needsAdmin = (batch.transactions || []).some((tx) =>
    (tx.commands || []).some((cmd) => cmd.administrator)
  );
  const activeTx = (batch.transactions || []).filter((tx) => (tx.commands || []).length > 0);

  body.appendChild(
    el(
      'div',
      { className: 'summary-strip' },
      metricPair('工具数量', `${totalTools} 项`),
      metricPair('后端事务', `${txCount} 组`),
      metricPair('系统权限', activeTx.length === 0 ? '无需提权' : needsAdmin ? '需要管理员授权' : '当前用户权限')
    )
  );

  if (activeTx.length === 0) {
    body.appendChild(
      el(
        'div',
        { className: 'callout is-ok' },
        el('div', { className: 'callout-title', text: '所选工具均已是最新' }),
        el('div', { text: '预检未发现待应用的版本更新，无需执行命令。' })
      )
    );
  }

  const txList = el('div', { className: 'record-list' });
  for (const tx of batch.transactions || []) {
    const names = (tx.toolIds || [])
      .map((id) => batch.plans.find((p) => p.tool.id === id)?.tool?.name || id)
      .join('、');
    const item = el(
      'div',
      { className: 'record-item' },
      el(
        'div',
        { className: 'record-card-head' },
        el('strong', { text: `${backendLabel(tx.backend)} · ${names}` }),
        el('span', {
          className: 'record-path tabular',
          text: (tx.commands || []).length > 0 ? `${tx.commands.length} 条命令` : '已最新',
        })
      )
    );
    for (const target of tx.preview?.targets || []) {
      const fromShort = target.installed ? formatShortVersion(target.installed) : '未安装';
      const toShort = target.candidate ? formatShortVersion(target.candidate) : '移除';
      item.appendChild(
        el('div', {
          className: 'record-path mono',
          text: `${target.name}: ${fromShort} → ${toShort}`,
          title: formatChangeText(target),
        })
      );
    }
    txList.appendChild(item);
  }
  body.appendChild(
    el('div', { className: 'panel-block' }, el('h3', { className: 'panel-label', text: '事务概览' }), txList)
  );

  const allCommands = (batch.transactions || []).flatMap((tx) => tx.commands || []);
  const allChanges = (batch.transactions || []).flatMap((tx) => tx.preview?.changes || []);
  const combinedBasis = [...new Set((batch.transactions || []).map((tx) => tx.preview?.basis).filter(Boolean))].join('；');
  const combinedDetails = (batch.transactions || [])
    .map((tx) => (tx.preview?.details ? `[${backendLabel(tx.backend)}]\n${tx.preview.details}` : ''))
    .filter(Boolean)
    .join('\n\n');

  body.appendChild(
    renderTechDetails(
      { basis: combinedBasis, changes: allChanges, details: combinedDetails },
      allCommands,
      modal
    )
  );
}

function metricPair(label, value) {
  return el(
    'div',
    { className: 'metric-pair' },
    el('span', { className: 'metric-label', text: label }),
    el('span', { className: 'metric-value', text: value })
  );
}

function formatChangeText(change) {
  const from = change.installed || '未安装';
  const to = change.candidate || '移除';
  const dep = change.dependency ? ' [依赖]' : '';
  return `${change.name}: ${from} → ${to}${dep}`;
}

function renderChangeList(title, changes) {
  const list = el('div', { className: 'record-list' });
  for (const change of changes) {
    const fromShort = change.installed ? formatShortVersion(change.installed) : '未安装';
    const toShort = change.candidate ? formatShortVersion(change.candidate) : '移除';
    list.appendChild(
      el(
        'div',
        { className: 'record-item' },
        el(
          'div',
          { className: 'record-card-head' },
          el('span', { className: 'mono', text: change.name }),
          el('span', {
            className: 'mono record-path',
            text: `${fromShort} → ${toShort}`,
            title: formatChangeText(change),
          })
        )
      )
    );
  }
  return el('div', { className: 'panel-block' }, el('h3', { className: 'panel-label', text: title }), list);
}

function renderTechDetails(preview, commands, modal) {
  const detailsBody = el('div', { className: 'tech-details-body' });

  if (preview?.basis) {
    detailsBody.appendChild(el('div', { className: 'record-path selectable', text: `预检依据：${preview.basis}` }));
  }

  if ((preview?.changes || []).length > 0) {
    const depChanges = preview.changes.filter((c) => c.dependency);
    if (depChanges.length > 0) {
      detailsBody.appendChild(
        el(
          'div',
          { className: 'panel-block' },
          el('h4', { className: 'panel-label', text: `连带依赖变更 (${depChanges.length})` }),
          el('pre', { className: 'code-block', text: depChanges.map(formatChangeText).join('\n') })
        )
      );
    }
  }

  if ((commands || []).length > 0) {
    const cmdList = el('div', { className: 'command-list' });
    for (const cmd of commands) {
      cmdList.appendChild(
        el(
          'div',
          { className: 'command-line' },
          el('span', { text: formatCommandLine(cmd) }),
          cmd.administrator ? el('span', { className: 'status-inline is-warn', text: '管理员' }) : null
        )
      );
    }
    detailsBody.appendChild(
      el(
        'div',
        { className: 'panel-block' },
        el('h4', { className: 'panel-label', text: `将执行的命令 (${commands.length})` }),
        cmdList
      )
    );
  }

  if (preview?.details) {
    detailsBody.appendChild(
      el(
        'div',
        { className: 'panel-block' },
        el('h4', { className: 'panel-label', text: '预检原始输出' }),
        el('pre', { className: 'code-block', text: preview.details })
      )
    );
  }

  return el(
    'details',
    {
      className: 'tech-details',
      open: Boolean(modal.techOpen),
      on: {
        toggle: (event) => {
          modal.techOpen = event.target.open;
        },
      },
    },
    el('summary', { text: '技术详情（预检依据、依赖与命令清单）' }),
    detailsBody
  );
}
