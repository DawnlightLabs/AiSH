import { copyFileSync, existsSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { arch, platform } from 'node:process';

function targetTriple() {
  const os = platform;
  const cpu = arch;
  if (os === 'win32') {
    return cpu === 'arm64' ? 'aarch64-pc-windows-msvc' : 'x86_64-pc-windows-msvc';
  }
  if (os === 'darwin') {
    return cpu === 'arm64' ? 'aarch64-apple-darwin' : 'x86_64-apple-darwin';
  }
  if (os === 'linux') {
    return cpu === 'arm64' ? 'aarch64-unknown-linux-gnu' : 'x86_64-unknown-linux-gnu';
  }
  throw new Error(`Unsupported platform for AiSH sidecar packaging: ${os}/${cpu}`);
}

const repoRoot = resolve(new URL('../..', import.meta.url).pathname);
const isWindows = platform === 'win32';
const source = join(repoRoot, 'target', 'release', isWindows ? 'aish.exe' : 'aish');
const outDir = join(repoRoot, 'apps', 'desktop', 'src-tauri', 'binaries');
const outName = `aish-provider-shell-${targetTriple()}${isWindows ? '.exe' : ''}`;
const target = join(outDir, outName);

if (!existsSync(source)) {
  throw new Error(`Provider shell binary not found at ${source}. Run npm run provider:build first.`);
}

mkdirSync(outDir, { recursive: true });
copyFileSync(source, target);
console.log(`Prepared AiSH provider sidecar: ${target}`);
