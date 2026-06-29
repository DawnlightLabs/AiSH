import fs from 'node:fs';
import path from 'node:path';

const source = path.resolve('..', 'site');
const output = path.resolve('site');

if (!fs.existsSync(source)) {
  throw new Error('Missing apps/site directory');
}

fs.rmSync(output, { recursive: true, force: true });
fs.mkdirSync(output, { recursive: true });
fs.cpSync(source, output, { recursive: true });

console.log('Copied AiSH site into desktop/site');
