import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { platform } from 'node:process';

function targetTriple() {
  try {
    return execSync('rustc --print host-tuple', { encoding: 'utf8' }).trim();
  } catch {
    const verbose = execSync('rustc -Vv', { encoding: 'utf8' });
    const host = verbose.split('\n').find((line) => line.startsWith('host:'));
    if (!host) {
      throw new Error('Could not determine Rust host target triple.');
    }
    return host.split(':')[1].trim();
  }
}

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..', '..');
const isWindows = platform === 'win32';
const source = join(repoRoot, 'target', 'release', isWindows ? 'aish.exe' : 'aish');
const outDir = join(repoRoot, 'apps', 'desktop', 'src-tauri', 'binaries');
const target = join(outDir, `aish-provider-shell-${targetTriple()}${isWindows ? '.exe' : ''}`);

if (!existsSync(source)) {
  throw new Error(`Provider shell binary not found at ${source}. Run npm run provider:build first.`);
}

mkdirSync(outDir, { recursive: true });
copyFileSync(source, target);
console.log(`Prepared AiSH provider sidecar: ${target}`);
