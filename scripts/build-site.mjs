import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const source = path.join(root, 'apps', 'site');
const output = path.join(root, 'site');

if (!fs.existsSync(source)) {
  throw new Error(`Missing landing site source directory: ${source}`);
}

fs.rmSync(output, { recursive: true, force: true });
fs.mkdirSync(output, { recursive: true });
fs.cpSync(source, output, { recursive: true });

console.log(`AiSH landing site copied to ${path.relative(root, output)}`);
