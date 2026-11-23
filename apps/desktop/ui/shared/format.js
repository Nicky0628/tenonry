export const BACKEND_NAMES = {
  apt: 'APT',
  dnf: 'DNF',
  pacman: 'pacman',
  winget: 'WinGet',
  npm: 'npm',
};

export const CATEGORY_LABELS = {
  languages: '语言与运行时',
  'package-managers': '包管理',
  'version-control': '版本控制',
  build: '构建与编译',
  containers: '容器与云原生',
  databases: '数据库客户端',
  cli: '终端与常用工具',
};

export const ACTION_LABELS = {
  install: '安装',
  update: '更新',
  uninstall: '卸载',
};

export const ERROR_KIND_LABELS = {
  invalidInput: '参数无效',
  notFound: '未找到目标',
  unsupported: '当前不支持',
  conflict: '来源或状态冲突',
  busy: '已有任务执行中',
  planExpired: '操作计划已过期',
  protected: '系统受保护组件',
  authorization: '需要系统授权',
  commandFailed: '命令执行失败',
  persistence: '记录保存异常',
  internal: '内部错误',
};

export const PHASE_LABELS = {
  checking: '复核计划',
  authorizing: '系统授权',
  downloading: '下载安装包',
  installing: '安装中',
  configuring: '配置环境',
  removing: '移除中',
  executing: '执行命令',
  verifying: '核验结果',
  finished: '已完成',
  interrupted: '已中断',
};

export const EXECUTION_LABELS = {
  notStarted: { label: '未启动', tone: 'neutral' },
  notNeeded: { label: '无需执行', tone: 'ok' },
  succeeded: { label: '执行成功', tone: 'ok' },
  failed: { label: '执行失败', tone: 'danger' },
  unknown: { label: '执行状态未知', tone: 'warn' },
};

export const VERIFICATION_LABELS = {
  passed: { label: '核验通过', tone: 'ok' },
  failed: { label: '核验未通过', tone: 'danger' },
  unknown: { label: '结果待确认', tone: 'warn' },
};

export function backendLabel(id) {
  return BACKEND_NAMES[id] || id || '未知后端';
}

export function categoryLabel(category) {
  return CATEGORY_LABELS[category] || category || '其他';
}

export function actionLabel(action) {
  return ACTION_LABELS[action] || action;
}

export function errorKindLabel(kind) {
  return ERROR_KIND_LABELS[kind] || '操作受阻';
}

export function phaseLabel(phase) {
  return PHASE_LABELS[phase] || phase || '处理中';
}

export function platformLabel(platform, arch) {
  const os = platform === 'windows' ? 'Windows' : platform === 'linux' ? 'Linux' : platform || '未知系统';
  return arch ? `${os} (${arch})` : os;
}

export function formatSourceBadge(tool) {
  if (!tool) return { label: '探测中', tone: 'neutral' };
  const source = tool.source;
  if (!source) return { label: tool.found ? '已发现' : '未安装', tone: tool.found ? 'ok' : 'neutral' };

  if (tool.state === 'system') {
    return { label: '系统受保护', tone: 'neutral' };
  }
  switch (source.kind) {
    case 'managed':
      return { label: `由 ${backendLabel(source.backend)} 管理`, tone: 'ok' };
    case 'mixed':
      return { label: `多来源冲突 · ${backendLabel(source.backend)}`, tone: 'warn' };
    case 'multiBackend': {
      const names = (source.backends || []).map(backendLabel).join(' / ');
      return { label: `多后端冲突 · ${names}`, tone: 'warn' };
    }
    case 'bundled':
      return { label: '随工具附带', tone: 'neutral' };
    case 'external':
      return { label: '外部安装', tone: 'neutral' };
    case 'unavailable':
      if (tool.state === 'missing') {
        return { label: '未安装', tone: 'neutral' };
      }
      return { label: '状态不可用', tone: 'danger' };
    default:
      return { label: tool.state === 'missing' ? '未安装' : '未知来源', tone: 'neutral' };
  }
}

export function formatExecutableSource(source) {
  if (!source) return { label: '未知', tone: 'neutral' };
  switch (source.kind) {
    case 'backend':
      return { label: backendLabel(source.backend), tone: 'ok' };
    case 'bundled':
      return { label: '附带组件', tone: 'neutral' };
    case 'versionManager':
      return { label: '版本管理器', tone: 'warn' };
    case 'external':
      return { label: '外部安装', tone: 'neutral' };
    default:
      return { label: '未知', tone: 'neutral' };
  }
}

export function formatOperationBadge(op) {
  if (!op) return { label: '未知', tone: 'neutral' };
  if (op.state === 'running') {
    return { label: phaseLabel(op.phase), tone: 'info' };
  }
  if (op.phase === 'interrupted') {
    return { label: '意外中断', tone: 'warn' };
  }
  if (op.state === 'completed') {
    return { label: '已完成', tone: 'ok' };
  }
  return { label: '需要关注', tone: 'warn' };
}

export function formatShortVersion(raw) {
  if (!raw) return '';
  const text = String(raw).trim();
  if (!text) return '';
  const firstLine = text.split(/\r?\n/, 1)[0].trim();
  const withoutEpoch = firstLine.replace(/^\d+:/, '');
  const java8 = withoutEpoch.match(/(?:1\.8\.0_|8u)(\d+)/);
  if (java8) return `8.0.${java8[1]}`;
  const semver = withoutEpoch.match(/(?<![\d.])(\d+\.\d+(?:\.\d+)?)/);
  if (semver) return semver[1];
  return firstLine;
}

export function extractCandidateVersion(preview) {
  if (!preview || !Array.isArray(preview.targets)) return null;
  const changed = preview.targets.find((t) => t.candidate && t.candidate !== t.installed);
  if (changed) return changed.candidate;
  const first = preview.targets.find((t) => t.candidate);
  return first ? first.candidate : null;
}

export function formatTimestamp(iso) {
  if (!iso) return '—';
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  const pad = (n) => String(n).padStart(2, '0');
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
  if (sameDay) return `今天 ${time}`;
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function formatDuration(ms) {
  const totalSeconds = Math.max(0, Math.floor((Number(ms) || 0) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes === 0) return `${seconds} 秒`;
  return `${minutes} 分 ${String(seconds).padStart(2, '0')} 秒`;
}

export function formatCommandLine(command) {
  const args = (command.arguments || []).map((arg) =>
    /[\s"'\\]/.test(arg) ? JSON.stringify(arg) : arg
  );
  return [command.program, ...args].join(' ');
}

const ERROR_PATTERNS = [
  {
    match: /0x8a15000f|-1978335217|缺少源所需的数据|source reset/i,
    summary: 'WinGet 软件源不可用，无法联网查询。',
    hint: '请检查网络连接（代理需开启 TUN 模式），或在终端执行 winget source update 初始化源索引。',
  },
  {
    match: /0x80072efd|0x80072ee7|0x80072ee2|无法与服务器建立连接|could not resolve|temporary failure resolving|failed to fetch|enotfound|etimedout|econnrefused|econnreset|network is unreachable|unable to access/i,
    summary: '无法连接软件源，请检查网络或代理设置。',
    hint: '确认网络畅通后重试。',
  },
  {
    match: /could not get lock|unable to lock|dpkg was interrupted|db\.lck|waiting for process|另一个.*正在运行/i,
    summary: '系统包管理器正被其他程序占用。',
    hint: '请等待其他安装或更新任务完成后重试。',
  },
  {
    match: /0x8a15002b|0x800704c7|dismissed|authorization.*cancel|已取消.*授权|退出码 126\b/i,
    summary: '已取消系统授权，未修改本机软件。',
    hint: '',
  },
  {
    match: /permission denied|are you root|eacces|eperm|需要管理员|缺少.*权限/i,
    summary: '缺少所需权限，无法执行该操作。',
    hint: '请确认当前账户具有相应权限后重试。',
  },
  {
    match: /0x8a150014|no package found|e404|unable to locate package|no match for argument|target not found/i,
    summary: '软件源中未找到对应的软件包或版本。',
    hint: '可先刷新软件源索引后再试。',
  },
  {
    match: /does not have a release file|404\s+not found|403\s+forbidden/i,
    summary: '软件源地址异常或暂不可访问。',
    hint: '请检查系统软件源配置或稍后重试。',
  },
  {
    match: /unmet dependencies|broken packages|conflicts with|依赖冲突/i,
    summary: '存在软件包依赖冲突，无法自动处理。',
    hint: '请先在终端或系统包管理器中解决依赖冲突。',
  },
  {
    match: /0x8a150011|hash.*mismatch|checksum|完整性校验/i,
    summary: '安装包完整性校验未通过。',
    hint: '请稍后重试或刷新软件源后再试。',
  },
  {
    match: /超时|timed?\s*out/i,
    summary: '查询响应超时，请稍后重试。',
    hint: '可能是网络较慢或软件源暂时无响应。',
  },
  {
    match: /无法启动/i,
    summary: '无法启动对应的命令行工具。',
    hint: '请确认包管理器已正确安装且在 PATH 中可用。',
  },
  {
    match: /检测输出过长/i,
    summary: '命令返回内容过长，已停止解析。',
    hint: '',
  },
];

/**
 * Turns a Tenon structured Error or raw backend message into a calm, concise
 * user-facing summary, optional next-step hint, and optional collapsible raw detail.
 */
export function formatErrorSummary(errorOrMessage) {
  const kind =
    errorOrMessage && typeof errorOrMessage === 'object' && typeof errorOrMessage.kind === 'string'
      ? errorOrMessage.kind
      : null;
  const raw = String(
    typeof errorOrMessage === 'string'
      ? errorOrMessage
      : errorOrMessage?.message || ''
  ).trim();

  const title = kind ? errorKindLabel(kind) : '';

  if (!raw) {
    return {
      title,
      summary: '操作未能完成，请重试。',
      hint: '',
      detail: '',
    };
  }

  for (const rule of ERROR_PATTERNS) {
    if (rule.match.test(raw)) {
      return {
        title,
        summary: rule.summary,
        hint: rule.hint,
        detail: raw !== rule.summary ? raw : '',
      };
    }
  }

  // Generic raw CLI command failure: collapse exit code + raw stdout/stderr into detail.
  if (/^命令未完成（退出码/.test(raw)) {
    return {
      title,
      summary: '包管理器命令执行未完成，请稍后重试。',
      hint: '可展开下方原始错误查看具体输出。',
      detail: raw,
    };
  }

  // Multi-line or overly verbose unstructured text: keep first concise line as summary.
  const lines = raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  if (lines.length > 1 || raw.length > 96) {
    const first = lines[0] || raw;
    const summary = first.length > 96 ? `${first.slice(0, 94)}…` : first;
    return {
      title,
      summary,
      hint: '',
      detail: raw,
    };
  }

  return {
    title,
    summary: raw,
    hint: '',
    detail: '',
  };
}
