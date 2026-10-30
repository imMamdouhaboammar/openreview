#!/usr/bin/env node
/** @license MIT · Deep Digger Build Script */
import { chmodSync, cpSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');
const distDir = join(rootDir, 'dist');

console.log('[Deep Digger] Building clean distribution bundle in dist/...');

// 1. Clean dist
if (existsSync(distDir)) {
  rmSync(distDir, { recursive: true, force: true });
}
mkdirSync(distDir, { recursive: true });

// 2. Copy CLI and postinstall scripts to dist
cpSync(join(rootDir, 'bin', 'cli.js'), join(distDir, 'cli.js'));
cpSync(join(rootDir, 'bin', 'postinstall.js'), join(distDir, 'postinstall.js'));
if (existsSync(join(rootDir, 'bin', 'minimizer.js'))) {
  cpSync(join(rootDir, 'bin', 'minimizer.js'), join(distDir, 'minimizer.js'));
  chmodSync(join(distDir, 'minimizer.js'), 0o755);
}
chmodSync(join(distDir, 'cli.js'), 0o755);
chmodSync(join(distDir, 'postinstall.js'), 0o755);

const filter = (src) => !src.endsWith('.DS_Store');

// 3. Copy python core engine
mkdirSync(join(distDir, 'scripts'), { recursive: true });
cpSync(join(rootDir, 'scripts', 'deep_digger.py'), join(distDir, 'scripts', 'deep_digger.py'));

// 4. Copy clean Rust source (excluding target/ and tests/)
mkdirSync(join(distDir, 'rust'), { recursive: true });
cpSync(join(rootDir, 'rust', 'Cargo.toml'), join(distDir, 'rust', 'Cargo.toml'));
if (existsSync(join(rootDir, 'rust', 'Cargo.lock'))) {
  cpSync(join(rootDir, 'rust', 'Cargo.lock'), join(distDir, 'rust', 'Cargo.lock'));
}
cpSync(join(rootDir, 'rust', 'src'), join(distDir, 'rust', 'src'), { recursive: true, filter });

// 5. Copy reference documentation, usecases, and solution specs
if (existsSync(join(rootDir, 'references'))) {
  cpSync(join(rootDir, 'references'), join(distDir, 'references'), { recursive: true, filter });
}
if (existsSync(join(rootDir, 'usecases'))) {
  cpSync(join(rootDir, 'usecases'), join(distDir, 'usecases'), { recursive: true, filter });
}
if (existsSync(join(rootDir, 'docs', 'solutions'))) {
  mkdirSync(join(distDir, 'docs'), { recursive: true });
  cpSync(join(rootDir, 'docs', 'solutions'), join(distDir, 'docs', 'solutions'), { recursive: true, filter });
}

// 6. Copy SKILL.md to dist as well
if (existsSync(join(rootDir, 'SKILL.md'))) {
  cpSync(join(rootDir, 'SKILL.md'), join(distDir, 'SKILL.md'));
}

console.log('[Deep Digger] Distribution bundle assembled successfully in dist/.');
