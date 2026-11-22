import { formatErrorSummary } from './format.js';

export function clear(node) {
  while (node.firstChild) {
    node.removeChild(node.firstChild);
  }
}

export function el(tag, options = {}, ...children) {
  const node = document.createElement(tag);
  if (options) {
    if (options.className) node.className = options.className;
    if (options.id) node.id = options.id;
    if (options.type) node.type = options.type;
    if (options.text != null) node.textContent = String(options.text);
    if (options.title != null) node.title = String(options.title);
    if (options.value != null) node.value = String(options.value);
    if (options.placeholder != null) node.placeholder = String(options.placeholder);
    if (options.disabled != null) node.disabled = Boolean(options.disabled);
    if (options.checked != null) node.checked = Boolean(options.checked);
    if (options.open != null) node.open = Boolean(options.open);
    if (options.attrs) {
      for (const [key, value] of Object.entries(options.attrs)) {
        if (value != null) node.setAttribute(key, String(value));
      }
    }
    if (options.dataset) {
      for (const [key, value] of Object.entries(options.dataset)) {
        if (value != null) node.dataset[key] = String(value);
      }
    }
    if (options.on) {
      for (const [event, handler] of Object.entries(options.on)) {
        if (typeof handler === 'function') {
          node.addEventListener(event, handler);
        }
      }
    }
  }
  appendChildren(node, children);
  return node;
}

function appendChildren(parent, children) {
  for (const child of children) {
    if (child == null || child === false) continue;
    if (Array.isArray(child)) {
      appendChildren(parent, child);
    } else if (child instanceof Node) {
      parent.appendChild(child);
    } else {
      parent.appendChild(document.createTextNode(String(child)));
    }
  }
}

export async function copyText(text) {
  if (!navigator.clipboard?.writeText) {
    return false;
  }
  try {
    await navigator.clipboard.writeText(String(text));
    return true;
  } catch {
    return false;
  }
}

export function renderErrorBlock(errorOrMessage, options = {}) {
  const info = formatErrorSummary(errorOrMessage);
  const heading = options.title || info.title || '';
  const tone = options.tone || 'is-danger';

  const children = [];
  if (heading) {
    children.push(el('div', { className: 'callout-title', text: heading }));
  }
  children.push(el('div', { text: info.summary }));
  if (info.hint) {
    children.push(el('div', { className: 'callout-hint', text: info.hint }));
  }
  if (info.detail) {
    children.push(
      el(
        'details',
        { className: 'error-raw' },
        el('summary', { text: '查看原始错误' }),
        el('pre', { className: 'code-block', text: info.detail })
      )
    );
  }
  return el('div', { className: `callout ${tone}` }, children);
}
