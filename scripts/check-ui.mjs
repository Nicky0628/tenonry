import { readdir, readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const uiRoot = fileURLToPath(new URL('../apps/desktop/ui/', import.meta.url));
const files = [];

async function walk(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const filename = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) {
      throw new Error(`界面资源不能使用符号链接：${filename}`);
    }
    if (entry.isDirectory()) {
      await walk(filename);
    } else if (entry.isFile()) {
      files.push(filename);
    }
  }
}

async function checkReference(from, reference) {
  if (reference.startsWith('#') || /^(?:data:|https?:)/.test(reference)) return;
  const clean = reference.split(/[?#]/, 1)[0];
  if (!clean) return;
  const target = path.resolve(path.dirname(from), clean);
  const relative = path.relative(uiRoot, target);
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error(`界面引用超出资源目录：${path.relative(uiRoot, from)} → ${reference}`);
  }
  if (!(await stat(target).catch(() => null))?.isFile()) {
    throw new Error(`找不到界面资源：${path.relative(uiRoot, from)} → ${reference}`);
  }
}

try {
  await walk(uiRoot);
  files.sort();
  let modules = 0;
  for (const filename of files) {
    if (filename.endsWith('.js')) {
      const result = spawnSync(process.execPath, ['--check', filename], {
        encoding: 'utf8',
        windowsHide: true,
      });
      if (result.error) throw result.error;
      if (result.status !== 0) throw new Error(result.stderr || `语法检查失败：${filename}`);
      const source = await readFile(filename, 'utf8');
      const imports = /\b(?:import\s+(?:(?:[^;'"\n]+?)\s+from\s*)?|export\s+[^;'"\n]+?\s+from\s*)['"]([^'"]+)['"]/g;
      for (const match of source.matchAll(imports)) {
        if (!match[1].startsWith('.')) {
          throw new Error(`浏览器模块必须使用相对导入：${filename} → ${match[1]}`);
        }
        await checkReference(filename, match[1]);
      }
      modules++;
    } else if (filename.endsWith('.html')) {
      const source = await readFile(filename, 'utf8');
      for (const match of source.matchAll(/\b(?:src|href)\s*=\s*["']([^"']+)["']/g)) {
        await checkReference(filename, match[1]);
      }
    }
  }
  console.log(`界面检查通过：${modules} 个 JavaScript 模块，资源引用有效。`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
