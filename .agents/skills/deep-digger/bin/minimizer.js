#!/usr/bin/env node
/** @license MIT · Deep Digger Minimizer */
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');

const binaryCandidates = [
  join(rootDir, 'rust', 'target', 'release', 'deep-digger-min'),
  join(rootDir, 'dist', 'rust', 'target', 'release', 'deep-digger-min'),
];

let nativeBin = null;
for (const cand of binaryCandidates) {
  if (existsSync(cand)) {
    nativeBin = cand;
    break;
  }
}

const args = process.argv.slice(2);

if (nativeBin) {
  const child = spawn(nativeBin, args, { stdio: 'inherit' });
  child.on('close', (code) => process.exit(code ?? 0));
} else {
  const pyScript = existsSync(join(__dirname, 'scripts', 'deep_digger.py'))
    ? join(__dirname, 'scripts', 'deep_digger.py')
    : join(rootDir, 'scripts', 'deep_digger.py');

  const child = spawn('python3', [pyScript, 'minimize', ...args], { stdio: 'inherit' });
  child.on('close', (code) => process.exit(code ?? 0));
}
