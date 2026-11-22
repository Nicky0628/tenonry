const SVG_NS = 'http://www.w3.org/2000/svg';

function svg(viewBox, className, nodes) {
  const root = document.createElementNS(SVG_NS, 'svg');
  root.setAttribute('viewBox', viewBox);
  root.setAttribute('fill', 'none');
  root.setAttribute('aria-hidden', 'true');
  if (className) root.setAttribute('class', className);
  for (const spec of nodes) {
    const child = document.createElementNS(SVG_NS, spec.tag);
    for (const [key, value] of Object.entries(spec)) {
      if (key === 'tag') continue;
      child.setAttribute(key, String(value));
    }
    root.appendChild(child);
  }
  return root;
}

export const TOOL_THEMES = {
  node: 'emerald',
  python: 'amber',
  git: 'orange',
  'git-lfs': 'coral',
  gh: 'slate',
  rust: 'copper',
  go: 'cyan',
  java: 'rose',
  pnpm: 'gold',
  dotnet: 'violet',
  cmake: 'sky',
  gcc: 'teal',
};

export function toolIcon(toolId, className = 'tool-emblem-svg') {
  switch (toolId) {
    case 'node':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M12 2.8 19.6 7.2v9.6L12 21.2 4.4 16.8V7.2L12 2.8Z', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M9.3 14.8V9.4l5.4 5.4V9.4', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'python':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M12 3.5c-3 0-4.2 1.2-4.2 3.3V9h5.6v1.1H6.6c-1.8 0-3.1 1.2-3.1 3.7s1.3 3.7 3.1 3.7h1.6v-2.1c0-1.9 1.4-3.3 3.4-3.3h3.6c1.5 0 2.6-1.1 2.6-2.7V6.8c0-2.1-1.6-3.3-5.8-3.3Z', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M12 20.5c3 0 4.2-1.2 4.2-3.3V15h-5.6v-1.1h6.8c1.8 0 3.1-1.2 3.1-3.7s-1.3-3.7-3.1-3.7h-1.6', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
        { tag: 'circle', cx: '10.1', cy: '6.3', r: '0.9', fill: 'currentColor' },
        { tag: 'circle', cx: '13.9', cy: '17.7', r: '0.9', fill: 'currentColor' },
      ]);
    case 'git':
      return svg('0 0 24 24', className, [
        { tag: 'circle', cx: '7.5', cy: '6.5', r: '2.2', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'circle', cx: '7.5', cy: '17.5', r: '2.2', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'circle', cx: '16.5', cy: '10', r: '2.2', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'path', d: 'M7.5 8.7v6.6M7.5 14.2c0-2.3 2.1-4.2 6.8-4.2', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round' },
      ]);
    case 'git-lfs':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M12 3.6 20 8l-8 4.4L4 8l8-4.4Z', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M4 12.2 12 16.6l8-4.4M4 16.2 12 20.6l8-4.4', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'gh':
      return svg('0 0 24 24', className, [
        { tag: 'rect', x: '3.5', y: '4.5', width: '17', height: '15', rx: '3.2', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'path', d: 'M8 10l2.8 2.4L8 14.8M12.5 15h3.5', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'rust':
      return svg('0 0 24 24', className, [
        { tag: 'circle', cx: '12', cy: '12', r: '6.4', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'path', d: 'M12 3.2v2.1M12 18.7v2.1M3.2 12h2.1M18.7 12h2.1M5.8 5.8l1.5 1.5M16.7 16.7l1.5 1.5M18.2 5.8l-1.5 1.5M7.3 16.7l-1.5 1.5', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round' },
        { tag: 'circle', cx: '12', cy: '12', r: '2.2', stroke: 'currentColor', 'stroke-width': '1.6' },
      ]);
    case 'go':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M4 9h4.5M3 12h4M4.5 15h4', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round' },
        { tag: 'rect', x: '10.2', y: '7', width: '10.5', height: '10', rx: '5', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'path', d: 'M15.8 12h4.7', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round' },
      ]);
    case 'java':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M6 10.5h10v4.8a4.2 4.2 0 0 1-4.2 4.2H10.2A4.2 4.2 0 0 1 6 15.3v-4.8Z', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M16 11.5h1.8a2.2 2.2 0 0 1 0 4.4H16M9.5 7.5c0-1.4 1.3-1.8 1.3-3.2M13 7.5c0-1.4 1.3-1.8 1.3-3.2', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round' },
      ]);
    case 'pnpm':
      return svg('0 0 24 24', className, [
        { tag: 'rect', x: '4', y: '4', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor' },
        { tag: 'rect', x: '9.7', y: '4', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor' },
        { tag: 'rect', x: '15.4', y: '4', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor' },
        { tag: 'rect', x: '9.7', y: '9.7', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor', opacity: '0.7' },
        { tag: 'rect', x: '15.4', y: '9.7', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor' },
        { tag: 'rect', x: '4', y: '15.4', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor', opacity: '0.5' },
        { tag: 'rect', x: '9.7', y: '15.4', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor', opacity: '0.7' },
        { tag: 'rect', x: '15.4', y: '15.4', width: '4.6', height: '4.6', rx: '1', fill: 'currentColor' },
      ]);
    case 'dotnet':
      return svg('0 0 24 24', className, [
        { tag: 'rect', x: '3.5', y: '5', width: '17', height: '14', rx: '3.5', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'circle', cx: '8', cy: '14.8', r: '1.1', fill: 'currentColor' },
        { tag: 'path', d: 'M11.2 15V9.2l4.8 5.8V9.2', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'cmake':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M12 3.8 20.2 19H3.8L12 3.8Z', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M12 3.8l1.6 9.5 6.6 5.7M12 13.3 3.8 19', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linejoin': 'round' },
      ]);
    case 'gcc':
      return svg('0 0 24 24', className, [
        { tag: 'path', d: 'M8.5 7.5 4 12l4.5 4.5M15.5 7.5 20 12l-4.5 4.5M13.3 6l-2.6 12', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    default:
      return svg('0 0 24 24', className, [
        { tag: 'rect', x: '4.5', y: '4.5', width: '15', height: '15', rx: '3.5', stroke: 'currentColor', 'stroke-width': '1.7' },
        { tag: 'circle', cx: '12', cy: '12', r: '2.5', fill: 'currentColor' },
      ]);
  }
}

export function uiIcon(name, className = 'ui-icon') {
  switch (name) {
    case 'copy':
      return svg('0 0 20 20', className, [
        { tag: 'rect', x: '7', y: '7', width: '9.5', height: '9.5', rx: '2', stroke: 'currentColor', 'stroke-width': '1.5' },
        { tag: 'path', d: 'M4.5 13H4a1.5 1.5 0 0 1-1.5-1.5V4A1.5 1.5 0 0 1 4 2.5h7.5A1.5 1.5 0 0 1 13 4v.5', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linecap': 'round' },
      ]);
    case 'check':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M4.5 10.5 8.2 14.2 15.8 6.4', stroke: 'currentColor', 'stroke-width': '1.9', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'refresh':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M15.8 10a5.8 5.8 0 1 1-1.5-3.9M15.8 4.2v2.9h-2.9', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'arrowUp':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M10 15V5M5.5 9.2 10 4.7l4.5 4.5', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'arrowRight':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M4.5 10h11M11.2 5.5 15.7 10l-4.5 4.5', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'download':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M10 4v8.5M6.5 9.2 10 12.7l3.5-3.5M4.5 15.5h11', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'layers':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M10 3.5 16.8 7 10 10.5 3.2 7 10 3.5Z', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M3.2 10.5 10 14l6.8-3.5M3.2 13.8 10 17.3l6.8-3.5', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'search':
      return svg('0 0 20 20', className, [
        { tag: 'circle', cx: '9', cy: '9', r: '5.5', stroke: 'currentColor', 'stroke-width': '1.6' },
        { tag: 'path', d: 'M13.2 13.2 16.8 16.8', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round' },
      ]);
    case 'close':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M5.5 5.5 14.5 14.5M14.5 5.5 5.5 14.5', stroke: 'currentColor', 'stroke-width': '1.7', 'stroke-linecap': 'round' },
      ]);
    case 'shield':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M10 2.8 16 5.2v4.6c0 3.7-2.5 6.4-6 7.5-3.5-1.1-6-3.8-6-7.5V5.2L10 2.8Z', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linejoin': 'round' },
      ]);
    case 'terminal':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M4.5 6.5 8 10l-3.5 3.5M10 13.5h5.5', stroke: 'currentColor', 'stroke-width': '1.6', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'trash':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M4.5 6h11M8 6V4.3c0-.4.3-.8.8-.8h2.4c.4 0 .8.3.8.8V6M6 6l.7 9.7c.1.7.6 1.3 1.3 1.3h4c.7 0 1.2-.6 1.3-1.3L14 6', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' },
      ]);
    case 'package':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M10 2.8 16.5 6.3v7.4L10 17.2 3.5 13.7V6.3L10 2.8Z', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linejoin': 'round' },
        { tag: 'path', d: 'M3.5 6.3 10 9.8l6.5-3.5M10 9.8v7.4', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linejoin': 'round' },
      ]);
    case 'sparkles':
      return svg('0 0 20 20', className, [
        { tag: 'path', d: 'M10 3.2 11.4 7.6 15.8 9 11.4 10.4 10 14.8 8.6 10.4 4.2 9 8.6 7.6 10 3.2Z', stroke: 'currentColor', 'stroke-width': '1.5', 'stroke-linejoin': 'round' },
      ]);
    default:
      return svg('0 0 20 20', className, [
        { tag: 'circle', cx: '10', cy: '10', r: '4', fill: 'currentColor' },
      ]);
  }
}
