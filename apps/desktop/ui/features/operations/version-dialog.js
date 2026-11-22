import { clear, el, renderErrorBlock } from '../../shared/dom.js';
import { backendLabel } from '../../shared/format.js';
import { uiIcon } from '../../shared/icons.js';

export function renderVersionDialog(host, modal, handlers) {
  const prevInput = host.querySelector('.version-search-input');
  const hadFocus = prevInput && document.activeElement === prevInput;
  const selStart = hadFocus ? prevInput.selectionStart : null;
  const selEnd = hadFocus ? prevInput.selectionEnd : null;

  clear(host);

  const tool = modal.tool;
  const action = (tool.actions || []).includes('install') ? 'install' : 'update';
  const versions = modal.data?.versions || [];
  const query = (modal.query || '').trim().toLowerCase();
  const filtered = query
    ? versions.filter(
        (item) =>
          item.version.toLowerCase().includes(query) ||
          (item.label && item.label.toLowerCase().includes(query))
      )
    : versions;

  const backendId = modal.data?.backend || modal.backend || tool.owner;

  const dialog = el('div', {
    className: 'modal-dialog',
    attrs: { role: 'dialog', 'aria-modal': 'true', 'aria-label': `选择 ${tool.name} 版本` },
  });

  dialog.appendChild(
    el(
      'div',
      { className: 'modal-header' },
      el(
        'div',
        null,
        el('h2', { className: 'modal-title', text: `选择 ${tool.name} 版本` }),
        el('p', {
          className: 'modal-subtitle',
          text: `${backendId ? `来源：${backendLabel(backendId)}` : '查询可用版本'} · 选定后将先生成可审阅计划`,
        })
      ),
      el(
        'button',
        {
          type: 'button',
          className: 'btn btn-ghost btn-sm',
          on: { click: () => handlers.closeModal() },
        },
        uiIcon('close'),
        '关闭'
      )
    )
  );

  const body = el('div', { className: 'modal-body' });

  if (modal.loading) {
    body.appendChild(
      el(
        'div',
        { className: 'loading-row' },
        el('span', { className: 'spinner' }),
        `正在向 ${backendId ? backendLabel(backendId) : '包管理器'} 查询可用版本列表…`
      )
    );
  } else if (modal.error) {
    body.appendChild(renderErrorBlock(modal.error));
  } else {
    if (modal.data?.message) {
      body.appendChild(el('div', { className: 'callout', text: modal.data.message }));
    }

    body.appendChild(
      el('input', {
        type: 'search',
        className: 'search-input version-search-input',
        value: modal.query || '',
        placeholder: '按版本号过滤…',
        attrs: { 'aria-label': '过滤版本号' },
        on: {
          input: (event) => handlers.setVersionQuery(event.target.value),
        },
      })
    );

    if (filtered.length === 0) {
      body.appendChild(
        el(
          'div',
          { className: 'empty-state' },
          el('p', {
            className: 'empty-state-title',
            text: versions.length === 0 ? '软件源未返回可安装版本' : '没有匹配的版本号',
          })
        )
      );
    } else {
      const list = el('div', { className: 'version-list', attrs: { role: 'listbox', 'aria-label': '版本列表' } });
      for (const item of filtered) {
        const selected = modal.selectedVersion === item.version;
        list.appendChild(
          el(
            'button',
            {
              type: 'button',
              className: `version-option${selected ? ' is-selected' : ''}`,
              attrs: { role: 'option', 'aria-selected': String(selected) },
              on: { click: () => handlers.selectVersion(item.version) },
            },
            el('span', { className: 'mono', text: item.label || item.version }),
            el(
              'span',
              { className: 'chip-list' },
              item.current ? el('span', { className: 'badge is-ok', text: '当前已安装' }) : null,
              selected ? el('span', { className: 'badge is-info', text: '已选' }) : null
            )
          )
        );
      }
      body.appendChild(list);
    }
  }

  const chosen = versions.find((item) => item.version === modal.selectedVersion);
  const canContinue = !modal.loading && !modal.error && Boolean(chosen) && !chosen.current;

  const footer = el(
    'div',
    { className: 'modal-footer' },
    el(
      'button',
      {
        type: 'button',
        className: 'btn',
        on: { click: () => handlers.closeModal() },
      },
      '取消'
    ),
    modal.error
      ? el(
          'button',
          {
            type: 'button',
            className: 'btn btn-primary',
            on: { click: () => handlers.reloadVersions() },
          },
          uiIcon('refresh'),
          '重试查询'
        )
      : el(
          'button',
          {
            type: 'button',
            className: 'btn btn-primary',
            disabled: !canContinue,
            on: {
              click: () =>
                handlers.openSinglePlan({
                  toolId: tool.id,
                  action,
                  backend: backendId || null,
                  version: modal.selectedVersion,
                }),
            },
          },
          chosen?.current ? '当前已是此版本' : '生成计划'
        )
  );

  dialog.appendChild(body);
  dialog.appendChild(footer);
  host.appendChild(dialog);

  if (hadFocus) {
    const input = host.querySelector('.version-search-input');
    if (input) {
      input.focus();
      if (selStart != null && selEnd != null) {
        input.setSelectionRange(selStart, selEnd);
      }
    }
  }
}
