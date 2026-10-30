#!/usr/bin/env node
/** @license MIT · Deep Digger Postinstall */
import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');
const distDir = existsSync(join(__dirname, 'scripts')) ? __dirname : join(rootDir, 'dist');

function copySkillFiles(targetPath) {
  mkdirSync(targetPath, { recursive: true });
  const sourceDir = existsSync(distDir) ? distDir : rootDir;
  cpSync(sourceDir, targetPath, {
    recursive: true,
    filter: (src) =>
      !src.includes('.git') &&
      !src.includes('node_modules') &&
      !src.includes('rust/target') &&
      !src.includes('evals') &&
      !src.includes('cases') &&
      !src.includes('__pycache__'),
  });
  // Ensure SKILL.md is present at target root
  if (!existsSync(join(targetPath, 'SKILL.md'))) {
    if (existsSync(join(rootDir, 'SKILL.md'))) {
      cpSync(join(rootDir, 'SKILL.md'), join(targetPath, 'SKILL.md'));
    }
  }
}

function installSkills() {
  console.log('[Deep Digger] Registering agent skill configuration...');

  // 1. Attempt npx / bunx skills add if online and registry tool is available
  try {
    const isBun = typeof process.versions.bun !== 'undefined';
    const runner = isBun ? 'bunx' : 'npx';
    const res = spawnSync(runner, ['skills', 'add', 'imMamdouhaboammar/deep-digger'], {
      stdio: 'pipe',
      timeout: 10000,
      encoding: 'utf-8',
    });
    if (res.status === 0) {
      console.log('  [PASS] Successfully registered via skills.sh registry.');
      return;
    }
  } catch (_) {
    // Fall back to direct directory linking
  }

  // 2. Direct linking fallback (offline-first, zero-network dependency)
  const home = homedir();
  const targets = [
    { name: 'Claude Code', path: join(home, '.claude', 'skills', 'deep-digger'), parent: join(home, '.claude') },
    { name: 'Antigravity / Gemini CLI', path: join(home, '.gemini', 'config', 'skills', 'deep-digger'), parent: join(home, '.gemini') },
    { name: 'Codex / OpenCode', path: join(home, '.codex', 'skills', 'deep-digger'), parent: join(home, '.codex') },
    { name: 'Universal Agent Kernel', path: join(home, '.agents', 'skills', 'deep-digger'), parent: join(home, '.agents') },
  ];

  let installedCount = 0;
  for (const target of targets) {
    try {
      if (existsSync(target.parent) || target.name === 'Universal Agent Kernel') {
        copySkillFiles(target.path);
        console.log(`  [PASS] Linked skill directly for ${target.name}.`);
        installedCount++;
      }
    } catch (_) {
      // Continue to next target if directory is inaccessible
    }
  }

  if (installedCount === 0) {
    try {
      const defaultTarget = join(home, '.agents', 'skills', 'deep-digger');
      copySkillFiles(defaultTarget);
      console.log('  [PASS] Linked skill to ~/.agents/skills/deep-digger.');
    } catch (_) {}
  }
}

function buildRust() {
  try {
    const cargoCheck = spawnSync('cargo', ['--version'], { stdio: 'pipe' });
    if (cargoCheck.status !== 0) {
      console.log('  [INFO] Rust compiler (cargo) not detected. Using built-in Python 3 engine.');
      return;
    }
    const cargoToml = existsSync(join(__dirname, 'rust', 'Cargo.toml'))
      ? join(__dirname, 'rust', 'Cargo.toml')
      : existsSync(join(rootDir, 'rust', 'Cargo.toml'))
      ? join(rootDir, 'rust', 'Cargo.toml')
      : null;

    if (!cargoToml) {
      console.log('  [INFO] Rust manifest not present. Using built-in Python 3 engine.');
      return;
    }

    console.log('[Deep Digger] Building native Rust engine (deep-digger-min)...');
    const build = spawnSync('cargo', ['build', '--release', '--manifest-path', cargoToml], {
      stdio: 'pipe',
      timeout: 60000,
      encoding: 'utf-8',
    });
    if (build.status === 0) {
      console.log('  [PASS] Native Rust engine compiled successfully.');
    } else {
      console.log('  [INFO] Native build skipped. Built-in Python 3 engine active.');
    }
  } catch (_) {
    console.log('  [INFO] Native build skipped. Built-in Python 3 engine active.');
  }
}

try {
  console.log('[Deep Digger] Running post-installation configuration...');
  installSkills();
  buildRust();
  console.log('[Deep Digger] Configuration complete.\n');
} catch (_) {
  // Fail-safe: never prevent package installation from succeeding
}
process.exit(0);
