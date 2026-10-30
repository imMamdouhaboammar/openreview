#!/usr/bin/env node
/** @license MIT · OpenReview Postinstall */
import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');
const distSkillsDir = existsSync(join(__dirname, 'skills')) ? join(__dirname, 'skills') : join(rootDir, 'skills');

function installAgentSkills() {
  console.log('[OpenReview] Auto-installing agent skills...');

  // 1. Primary: Run npx / bunx skills add
  try {
    const isBun = typeof process.versions.bun !== 'undefined';
    const runner = isBun ? 'bunx' : 'npx';
    console.log(`[OpenReview] Executing ${runner} skills add imMamdouhaboammar/openreview...`);
    const res = spawnSync(runner, ['skills', 'add', 'imMamdouhaboammar/openreview'], {
      stdio: 'inherit',
      timeout: 20000,
    });
    if (res.status === 0) {
      console.log('  [PASS] Registered OpenReview skills via registry.');
      return;
    }
  } catch (_) {
    // Fall back to direct directory linking
  }

  // 2. Direct offline-first agent linking fallback
  if (!existsSync(distSkillsDir)) {
    return;
  }

  const home = homedir();
  const targetAgentDirs = [
    { name: 'Claude Code', path: join(home, '.claude', 'skills') },
    { name: 'Antigravity / Gemini CLI', path: join(home, '.gemini', 'config', 'skills') },
    { name: 'Codex / OpenCode', path: join(home, '.codex', 'skills') },
    { name: 'Universal Agent Kernel', path: join(home, '.agents', 'skills') },
  ];

  const skillNames = ['openreview-orchestrator', 'deep-digger', 'openreview'];

  for (const agent of targetAgentDirs) {
    try {
      if (existsSync(dirname(agent.path)) || agent.name === 'Universal Agent Kernel') {
        mkdirSync(agent.path, { recursive: true });
        for (const skill of skillNames) {
          const src = join(distSkillsDir, skill);
          const dest = join(agent.path, skill);
          if (existsSync(src)) {
            cpSync(src, dest, {
              recursive: true,
              filter: (p) => !p.includes('.git') && !p.includes('node_modules') && !p.includes('target'),
            });
          }
        }
        console.log(`  [PASS] Linked skills directly for ${agent.name}.`);
      }
    } catch (_) {
      // Continue safely
    }
  }
}

function compileNativeService() {
  const cargoToml = existsSync(join(rootDir, 'Cargo.toml')) ? join(rootDir, 'Cargo.toml') : null;
  if (!cargoToml) {
    return;
  }

  try {
    const cargoCheck = spawnSync('cargo', ['--version'], { stdio: 'pipe' });
    if (cargoCheck.status !== 0) {
      return;
    }

    console.log('[OpenReview] Compiling native Rust service binary...');
    const build = spawnSync('cargo', ['build', '--release'], {
      cwd: rootDir,
      stdio: 'inherit',
      timeout: 120000,
    });
    if (build.status === 0) {
      console.log('  [PASS] Native OpenReview binary compiled successfully.');
    }
  } catch (_) {
    // Continue safely
  }
}

try {
  console.log('[OpenReview] Running post-installation configuration...');
  installAgentSkills();
  compileNativeService();
  console.log('[OpenReview] Postinstall configuration complete.\n');
} catch (_) {
  // Fail-safe: never prevent package installation from completing
}
process.exit(0);
