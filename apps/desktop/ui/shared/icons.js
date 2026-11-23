const SVG_NS = 'http://www.w3.org/2000/svg';

function renderSvg(viewBox, className, shapes, strokeWidth = '1.65') {
  const root = document.createElementNS(SVG_NS, 'svg');
  root.setAttribute('viewBox', viewBox);
  root.setAttribute('fill', 'none');
  root.setAttribute('stroke', 'currentColor');
  root.setAttribute('stroke-width', strokeWidth);
  root.setAttribute('stroke-linecap', 'round');
  root.setAttribute('stroke-linejoin', 'round');
  root.setAttribute('aria-hidden', 'true');
  if (className) root.setAttribute('class', className);

  for (const item of Array.isArray(shapes) ? shapes : [shapes]) {
    if (typeof item === 'string') {
      const node = document.createElementNS(SVG_NS, 'path');
      node.setAttribute('d', item);
      root.appendChild(node);
      continue;
    }
    const [kind, ...args] = item;
    let node = null;
    if (kind === 'c' || kind === 'fc') {
      node = document.createElementNS(SVG_NS, 'circle');
      node.setAttribute('cx', String(args[0]));
      node.setAttribute('cy', String(args[1]));
      node.setAttribute('r', String(args[2]));
      if (kind === 'fc') {
        node.setAttribute('fill', 'currentColor');
        node.setAttribute('stroke', 'none');
      }
    } else if (kind === 'r' || kind === 'fr') {
      node = document.createElementNS(SVG_NS, 'rect');
      node.setAttribute('x', String(args[0]));
      node.setAttribute('y', String(args[1]));
      node.setAttribute('width', String(args[2]));
      node.setAttribute('height', String(args[3]));
      if (args[4] != null) node.setAttribute('rx', String(args[4]));
      if (kind === 'fr') {
        node.setAttribute('fill', 'currentColor');
        node.setAttribute('stroke', 'none');
        if (args[5] != null) node.setAttribute('opacity', String(args[5]));
      }
    } else if (kind === 'e') {
      node = document.createElementNS(SVG_NS, 'ellipse');
      node.setAttribute('cx', String(args[0]));
      node.setAttribute('cy', String(args[1]));
      node.setAttribute('rx', String(args[2]));
      node.setAttribute('ry', String(args[3]));
    } else if (kind === 'p') {
      node = document.createElementNS(SVG_NS, 'polygon');
      node.setAttribute('points', String(args[0]));
    }
    if (node) root.appendChild(node);
  }
  return root;
}

export const TOOL_THEMES = {
  python: 'amber', node: 'emerald', typescript: 'sky', rust: 'copper', go: 'cyan', java: 'rose',
  dotnet: 'violet', ruby: 'rose', php: 'violet', zig: 'gold', bun: 'amber', deno: 'slate',
  pnpm: 'gold', yarn: 'sky', uv: 'violet', pipx: 'teal', poetry: 'sky', composer: 'copper',
  git: 'orange', 'git-lfs': 'coral', gh: 'slate', glab: 'orange', lazygit: 'emerald', delta: 'cyan',
  cmake: 'sky', gcc: 'teal', clang: 'cyan', make: 'slate', ninja: 'rose', meson: 'violet',
  maven: 'coral', gdb: 'amber', just: 'gold', protoc: 'teal',
  docker: 'sky', podman: 'violet', kubectl: 'sky', helm: 'cyan', terraform: 'violet', ansible: 'rose',
  sqlite: 'sky', psql: 'cyan', mysql: 'teal', 'redis-cli': 'rose',
  neovim: 'emerald', vim: 'emerald', tmux: 'teal', ripgrep: 'orange', fd: 'amber', bat: 'violet',
  jq: 'gold', fzf: 'coral', curl: 'cyan', httpie: 'emerald', shellcheck: 'teal', shfmt: 'sky',
  direnv: 'amber', starship: 'rose', zoxide: 'orange', eza: 'cyan',
};

const TOOL_ICONS = {
  python: [
    'M12 3.5c-3 0-4.2 1.2-4.2 3.3V9h5.6v1.1H6.6c-1.8 0-3.1 1.2-3.1 3.7s1.3 3.7 3.1 3.7h1.6v-2.1c0-1.9 1.4-3.3 3.4-3.3h3.6c1.5 0 2.6-1.1 2.6-2.7V6.8c0-2.1-1.6-3.3-5.8-3.3Z',
    'M12 20.5c3 0 4.2-1.2 4.2-3.3V15h-5.6v-1.1h6.8c1.8 0 3.1-1.2 3.1-3.7s-1.3-3.7-3.1-3.7h-1.6',
    ['fc', 10.1, 6.3, 0.9], ['fc', 13.9, 17.7, 0.9],
  ],
  node: 'M12 2.8 19.6 7.2v9.6L12 21.2 4.4 16.8V7.2L12 2.8ZM9.3 14.8V9.4l5.4 5.4V9.4',
  typescript: [['r', 3.5, 3.5, 17, 17, 3.2], 'M7 10.2h5.2M9.6 10.2v6.6M17 11.4c-.4-.8-1.1-1.2-2-1.2-1.1 0-1.9.6-1.9 1.5 0 2.1 4.1 1.2 4.1 3.5 0 1-.9 1.7-2.2 1.7-1.1 0-1.9-.5-2.3-1.2'],
  rust: [['c', 12, 12, 6.4], ['c', 12, 12, 2.2], 'M12 3.2v2.1M12 18.7v2.1M3.2 12h2.1M18.7 12h2.1M5.8 5.8l1.5 1.5M16.7 16.7l1.5 1.5M18.2 5.8l-1.5 1.5M7.3 16.7l-1.5 1.5'],
  go: [['r', 10.2, 7, 10.5, 10, 5], 'M4 9h4.5M3 12h4M4.5 15h4M15.8 12h4.7'],
  java: 'M6 10.5h10v4.8a4.2 4.2 0 0 1-4.2 4.2H10.2A4.2 4.2 0 0 1 6 15.3v-4.8ZM16 11.5h1.8a2.2 2.2 0 0 1 0 4.4H16M9.5 7.5c0-1.4 1.3-1.8 1.3-3.2M13 7.5c0-1.4 1.3-1.8 1.3-3.2',
  dotnet: [['r', 3.5, 5, 17, 14, 3.5], ['fc', 8, 14.8, 1.1], 'M11.2 15V9.2l4.8 5.8V9.2'],
  ruby: 'M7.5 4.8h9L20.5 9.6 12 19.6 3.5 9.6l4-4.8ZM3.5 9.6h17M12 19.6 8.5 9.6l3.5-4.8 3.5 4.8-3.5 10Z',
  php: [['e', 12, 12, 9, 5.8], 'M7.5 14v-4h1.6a1.1 1.1 0 0 1 0 2.2H7.5M12 10v4M12 12h1.8a1 1 0 0 1 1 1v1M16.8 14v-4h1.5a1.1 1.1 0 0 1 0 2.2h-1.5'],
  zig: 'M5.5 6.5h13l-9.5 11h10M4.5 10.5 8 6.5M16 17.5l3.5-4',
  bun: ['M12 5.2c4.8 0 8.2 3.1 8.2 7.2 0 3.7-3.4 6.4-8.2 6.4s-8.2-2.7-8.2-6.4c0-4.1 3.4-7.2 8.2-7.2ZM10.8 15c.6.6 1.8.6 2.4 0', ['fc', 9.4, 12.5, 1], ['fc', 14.6, 12.5, 1]],
  deno: [['c', 12, 12, 8.2], 'M9 19.2c.2-3.5 1.1-5.6 3.2-6.4 1.5-.6 3.3-.6 4.2-1.7.7-.8.5-2.1-.6-2.7-1.6-.9-4.5-.7-6 1.2', ['fc', 13.4, 9.6, 0.9]],
  pnpm: [
    ['fr', 4, 4, 4.6, 4.6, 1], ['fr', 9.7, 4, 4.6, 4.6, 1], ['fr', 15.4, 4, 4.6, 4.6, 1],
    ['fr', 9.7, 9.7, 4.6, 4.6, 1, 0.7], ['fr', 15.4, 9.7, 4.6, 4.6, 1],
    ['fr', 4, 15.4, 4.6, 4.6, 1, 0.5], ['fr', 9.7, 15.4, 4.6, 4.6, 1, 0.7], ['fr', 15.4, 15.4, 4.6, 4.6, 1],
  ],
  yarn: [['c', 12, 12, 7.8], 'M6.2 9.5c3.5.4 6.5 2.5 9.2 6.8M9.5 4.8c2.8 2.4 5.4 6 6.8 11.4M5 14.2c3.8-1.6 7.8-1.6 13.4-.3'],
  uv: 'M5.5 7v6.2a3.3 3.3 0 0 0 6.6 0V7M13.5 7l2.8 9.5L19.2 7',
  pipx: 'M12 3.5 19.5 7.8v8.4L12 20.5 4.5 16.2V7.8L12 3.5ZM9.2 9.2 14.8 14.8M14.8 9.2l-5.6 5.6',
  poetry: 'M5 18.8 17.8 6c1.1 3.6.3 7.6-2.5 10.4-2.5 2.5-6.2 3.3-10.3 2.4ZM5 18.8 13.5 10.3',
  composer: ['M10 16.5V6.8l8.5-2v9.5', ['c', 7.5, 16.5, 2.5], ['c', 16, 14.5, 2.5]],
  git: [['c', 7.5, 6.5, 2.2], ['c', 7.5, 17.5, 2.2], ['c', 16.5, 10, 2.2], 'M7.5 8.7v6.6M7.5 14.2c0-2.3 2.1-4.2 6.8-4.2'],
  'git-lfs': 'M12 3.6 20 8l-8 4.4L4 8l8-4.4ZM4 12.2 12 16.6l8-4.4M4 16.2 12 20.6l8-4.4',
  gh: [['r', 3.5, 4.5, 17, 15, 3.2], 'M8 10l2.8 2.4L8 14.8M12.5 15h3.5'],
  glab: 'M12 19.6 4.2 13.8l2.2-8.4 2.8 6.4h5.6l2.8-6.4 2.2 8.4L12 19.6Z',
  lazygit: [['r', 3.5, 4, 17, 16, 2.8], 'M9.5 4v16M9.5 11.5h11', ['fc', 6.5, 8.5, 1.1], ['fc', 6.5, 15.5, 1.1]],
  delta: 'M12 4.2 20 18.8H4L12 4.2ZM12 9.5v5.8',
  cmake: 'M12 3.8 20.2 19H3.8L12 3.8ZM12 3.8l1.6 9.5 6.6 5.7M12 13.3 3.8 19',
  gcc: 'M8.5 7.5 4 12l4.5 4.5M15.5 7.5 20 12l-4.5 4.5M13.3 6l-2.6 12',
  clang: 'M17.5 7.6A7.3 7.3 0 1 0 17.5 16.4M13 10.2 17.5 12 13 13.8',
  make: 'M14.5 5.5 18.5 9.5l-2.3 2.3-4-4 2.3-2.3ZM12.2 7.8 5.2 14.8a1.9 1.9 0 0 0 2.7 2.7l7-7',
  ninja: ['M12 3.5 14.4 9.6 20.5 12l-6.1 2.4L12 20.5l-2.4-6.1L3.5 12l6.1-2.4L12 3.5Z', ['fc', 12, 12, 1.6]],
  meson: [['p', '12,3.4 19.6,7.7 19.6,16.3 12,20.6 4.4,16.3 4.4,7.7'], 'M8 15V9.5l4 3.4 4-3.4V15'],
  maven: 'M4.5 17.5V6.5l4.2 6.8 3.3-5.2 3.3 5.2 4.2-6.8v11',
  gdb: [['c', 12, 13, 5], 'M9.5 8.5a2.5 2.5 0 0 1 5 0M4.5 13h2.5M17 13h2.5M5.8 9.2l2 1.2M18.2 9.2l-2 1.2M5.8 16.8l2-1.2M18.2 16.8l-2-1.2M12 8v10'],
  just: 'M6 7h12M6 12h8M6 17h6M14.8 14.5l2 2 3.2-3.6',
  protoc: [['r', 4, 5, 6.5, 5.5, 1.4], ['r', 13.5, 13.5, 6.5, 5.5, 1.4], 'M10.5 7.8h4.2a2 2 0 0 1 2 2v3.7M13.5 16.2H9.3a2 2 0 0 1-2-2v-3.7'],
  docker: 'M4 13.5h15.5c-.6 3.5-3.4 5.5-7.8 5.5-4.3 0-7.7-2-7.7-5.5ZM7 10.2h2.4v2.3H7zM10.5 10.2h2.4v2.3h-2.4zM14 10.2h2.4v2.3H14zM10.5 6.8h2.4v2.3h-2.4z',
  podman: [['c', 8.5, 11.5, 3.2], ['c', 15.5, 11.5, 3.2], 'M4.5 18c1.2-1.8 3.2-2.6 7.5-2.6s6.3.8 7.5 2.6'],
  kubectl: [['p', '12,3.4 19,7.4 20.2,15.2 15.2,20.2 8.8,20.2 3.8,15.2 5,7.4'], ['c', 12, 12.2, 2.2], 'M12 6.5v3.5M16.8 14.5l-2.9-1.4M7.2 14.5l2.9-1.4'],
  helm: [['c', 12, 12, 5.5], ['c', 12, 12, 1.8], 'M12 3.5v3M12 17.5v3M3.5 12h3M17.5 12h3M6 6l2.1 2.1M15.9 15.9 18 18M18 6l-2.1 2.1M8.1 15.9 6 18'],
  terraform: 'M4.5 5.2 9.5 8v5.5l-5-2.8V5.2ZM10.8 8.7l5 2.8V17l-5-2.8V8.7ZM16.8 8.3l3.2-1.8v5.5l-3.2 1.8V8.3ZM10.8 15.6l5 2.8v2.6l-5-2.8v-2.6Z',
  ansible: [['c', 12, 12, 8.2], 'M8.5 16.5 12.2 7.2l3.8 9.3-5.8-4.2'],
  sqlite: [['e', 12, 6.8, 6.8, 2.6], 'M5.2 6.8v10.4c0 1.4 3 2.6 6.8 2.6s6.8-1.2 6.8-2.6V6.8M5.2 12c0 1.4 3 2.6 6.8 2.6s6.8-1.2 6.8-2.6'],
  psql: [['e', 11, 6.5, 6, 2.4], 'M5 6.5v9.8c0 1.3 2.7 2.4 6 2.4s6-1.1 6-2.4V6.5M17 11.5c1.8 0 2.8 1.2 2.8 3.2 0 1.6-.8 2.8-2.2 3.3'],
  mysql: [['e', 12, 6.8, 6.8, 2.5], 'M5.2 6.8v10.4c0 1.4 3 2.5 6.8 2.5s6.8-1.1 6.8-2.5V6.8M8.5 13.8c1.1-1.3 2.4-2 3.7-2 1.2 0 2.3.7 3.3 2'],
  'redis-cli': 'M12 4.2 19.6 8 12 11.8 4.4 8 12 4.2ZM4.4 12 12 15.8 19.6 12M4.4 16 12 19.8 19.6 16',
  neovim: 'M6 19V5.2l3.5 3.2V19M18 5v13.8l-3.5-3.2V5M6.8 5.8l10.4 12.4',
  vim: 'M12 3.5 20.5 12 12 20.5 3.5 12 12 3.5ZM8.8 8.8 12 15.8l3.2-7',
  tmux: [['r', 3.5, 4.5, 17, 15, 2.6], 'M12 4.5v11.5M12 10.5h8.5M3.5 16h17'],
  ripgrep: [['c', 10.5, 10.5, 5.8], 'm15 15 4.8 4.8M8.2 10.5h4.6M10.5 8.2v4.6'],
  fd: ['M4 7.5a2 2 0 0 1 2-2h4.2l1.8 2h6a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-9Z', ['c', 12, 13, 2.2]],
  bat: 'M6 4h8.5L19 8.5V19a1.5 1.5 0 0 1-1.5 1.5H6.5A1.5 1.5 0 0 1 5 19V5.5A1.5 1.5 0 0 1 6 4ZM8.5 11.5h7M8.5 15h5',
  jq: ['M8.8 5.2C7 5.2 6.5 6.4 6.5 8v2c0 1.2-.8 2-2 2 1.2 0 2 .8 2 2v2c0 1.6.5 2.8 2.3 2.8M15.2 5.2c1.8 0 2.3 1.2 2.3 2.8v2c0 1.2.8 2 2 2-1.2 0-2 .8-2 2v2c0 1.6-.5 2.8-2.3 2.8', ['fc', 12, 12, 1.3]],
  fzf: 'M4.5 6.5h15l-5.8 6.4v4.9l-3.4 1.7v-6.6L4.5 6.5Z',
  curl: 'M5 9.5h11.5M13 6l3.8 3.5L13 13M19 14.5H7.5M11 11l-3.8 3.5L11 18',
  httpie: [['c', 12, 12, 8], 'M4.3 12h15.4M12 4c2.3 2.3 3.4 5 3.4 8s-1.1 5.7-3.4 8c-2.3-2.3-3.4-5-3.4-8s1.1-5.7 3.4-8Z'],
  shellcheck: 'M12 3.5 19 6.3v5.4c0 4.3-2.9 7.4-7 8.8-4.1-1.4-7-4.5-7-8.8V6.3L12 3.5ZM9 12.2l2 2 4.2-4.4',
  shfmt: ['M5 6.5h14M5 11.5h9.5M8.5 16.5h10.5', ['fc', 5.5, 16.5, 1.1]],
  direnv: 'M4 7.5a2 2 0 0 1 2-2h4.2l1.8 2h6a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-9ZM9 12.2h6M9 15h4',
  starship: 'M12 3.6 14.3 9l5.7.6-4.3 3.7 1.3 5.6L12 16l-5 2.9 1.3-5.6L4 9.6 9.7 9 12 3.6Z',
  zoxide: 'M6 6.5h12l-12 11h12M15.2 14.8l2.8 2.7-2.8 2.5',
  eza: [['fc', 6, 7, 1.2], ['fc', 6, 12, 1.2], ['fc', 6, 17, 1.2], 'M10 7h9M10 12h9M10 17h6.5'],
};

const DEFAULT_TOOL_ICON = [['r', 4.5, 4.5, 15, 15, 3.5], ['fc', 12, 12, 2.5]];

const UI_ICONS = {
  copy: [['r', 7, 7, 9.5, 9.5, 2], 'M4.5 13H4a1.5 1.5 0 0 1-1.5-1.5V4A1.5 1.5 0 0 1 4 2.5h7.5A1.5 1.5 0 0 1 13 4v.5'],
  check: 'M4.5 10.5 8.2 14.2 15.8 6.4',
  refresh: 'M15.8 10a5.8 5.8 0 1 1-1.5-3.9M15.8 4.2v2.9h-2.9',
  arrowUp: 'M10 15V5M5.5 9.2 10 4.7l4.5 4.5',
  arrowRight: 'M4.5 10h11M11.2 5.5 15.7 10l-4.5 4.5',
  download: 'M10 4v8.5M6.5 9.2 10 12.7l3.5-3.5M4.5 15.5h11',
  layers: 'M10 3.5 16.8 7 10 10.5 3.2 7 10 3.5ZM3.2 10.5 10 14l6.8-3.5M3.2 13.8 10 17.3l6.8-3.5',
  search: [['c', 9, 9, 5.5], 'M13.2 13.2 16.8 16.8'],
  close: 'M5.5 5.5 14.5 14.5M14.5 5.5 5.5 14.5',
  shield: 'M10 2.8 16 5.2v4.6c0 3.7-2.5 6.4-6 7.5-3.5-1.1-6-3.8-6-7.5V5.2L10 2.8Z',
  terminal: 'M4.5 6.5 8 10l-3.5 3.5M10 13.5h5.5',
  trash: 'M4.5 6h11M8 6V4.3c0-.4.3-.8.8-.8h2.4c.4 0 .8.3.8.8V6M6 6l.7 9.7c.1.7.6 1.3 1.3 1.3h4c.7 0 1.2-.6 1.3-1.3L14 6',
  package: 'M10 2.8 16.5 6.3v7.4L10 17.2 3.5 13.7V6.3L10 2.8ZM3.5 6.3 10 9.8l6.5-3.5M10 9.8v7.4',
  sparkles: 'M10 3.2 11.4 7.6 15.8 9 11.4 10.4 10 14.8 8.6 10.4 4.2 9 8.6 7.6 10 3.2Z',
};

export function toolIcon(toolId, className = 'tool-emblem-svg') {
  return renderSvg('0 0 24 24', className, TOOL_ICONS[toolId] || DEFAULT_TOOL_ICON);
}

export function uiIcon(name, className = 'ui-icon') {
  return renderSvg('0 0 20 20', className, UI_ICONS[name] || [['fc', 10, 10, 4]], '1.6');
}
