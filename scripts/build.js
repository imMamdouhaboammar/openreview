#!/usr/bin/env node
/** @license MIT · OpenReview Build Script */
import { chmodSync, cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');
const distDir = join(rootDir, 'dist');

console.log('[OpenReview] Assembling production distribution bundle in dist/...');

// 1. Clean dist
if (existsSync(distDir)) {
  rmSync(distDir, { recursive: true, force: true });
}
mkdirSync(distDir, { recursive: true });

// 2. Copy CLI and postinstall scripts to dist
cpSync(join(rootDir, 'bin', 'cli.js'), join(distDir, 'cli.js'));
cpSync(join(rootDir, 'bin', 'postinstall.js'), join(distDir, 'postinstall.js'));
chmodSync(join(distDir, 'cli.js'), 0o755);
chmodSync(join(distDir, 'postinstall.js'), 0o755);

const filter = (src) =>
  !src.includes('.git') &&
  !src.includes('target') &&
  !src.includes('node_modules') &&
  !src.includes('.DS_Store') &&
  !src.includes('__pycache__');

// 3. Copy bundled skills to dist/skills for direct offline registration
if (existsSync(join(rootDir, 'skills'))) {
  mkdirSync(join(distDir, 'skills'), { recursive: true });
  cpSync(join(rootDir, 'skills'), join(distDir, 'skills'), { recursive: true, filter });
}

console.log('[OpenReview] Distribution bundle assembled successfully in dist/.');
