#!/usr/bin/env node
/** @license MIT · OpenReview */
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');

const VERSION = '0.1.0';

function findBinary() {
  const candidates = [
    join(rootDir, 'target', 'release', 'openreview'),
    join(rootDir, 'target', 'debug', 'openreview'),
    join(__dirname, 'openreview'),
    join(homedir(), '.cargo', 'bin', 'openreview'),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

function showBanner() {
  console.log(`
OpenReview v${VERSION}
Evidence-Gated AI Code Review Bot, Microservice & Investigation Workbench

Usage:
  openreview <command> [options]

Commands:
  review --repo <OWNER/REPO> --pr <NUMBER> [--dry-run]
      Execute read-only pull request review with 3-tier severity classification
  
  serve [--host <HOST>] [--port <PORT>]
      Start the multi-tenant webhook daemon service (HTTP/REST)
  
  action [--dry-run]
      Run inside GitHub Actions runner using event payload
  
  verify-config
      Validate environment variables, GitHub App credentials, and LLM connectivity
  
  doctor
      Run preflight health checks (Bun/Node, Rust, Cargo, Docker, Python 3)
  
  skills [install|list]
      Inspect or register OpenReview agent skills (orchestrator, deep-digger)
  
  version
      Print current version

Integration & Agent Skills:
  Skills Registry:  npx skills add imMamdouhaboammar/openreview
  Package Manager:  npm install -g openreview-ai (or bun add -g openreview-ai)
  GitHub Action:    uses: imMamdouhaboammar/openreview@main
`);
}

async function runDoctor() {
  console.log(`\n[OpenReview] Running System Preflight Diagnostics...\n`);
  const checks = [
    { name: 'JavaScript Runtime', check: () => `${process.release?.name || 'node'} ${process.version}` },
    {
      name: 'Rust Compiler (cargo)',
      cmd: 'cargo',
      args: ['--version'],
      required: false,
    },
    {
      name: 'Python 3 Standard Library',
      cmd: 'python3',
      args: ['--version'],
      required: false,
    },
    {
      name: 'Docker Daemon (for Fix Sandboxing)',
      cmd: 'docker',
      args: ['--version'],
      required: false,
    },
    {
      name: 'Git CLI',
      cmd: 'git',
      args: ['--version'],
      required: true,
    },
    {
      name: 'Native Binary',
      check: () => {
        const bin = findBinary();
        return bin ? `Compiled (${bin})` : 'Not compiled (runs via cargo or Docker container)';
      },
      required: false,
    },
  ];

  let hasFatal = false;
  for (const c of checks) {
    if (c.check) {
      const res = c.check();
      if (res) {
        console.log(`  [PASS] ${c.name}: ${res}`);
      } else {
        console.log(`  [FAIL] ${c.name}: Not found`);
        if (c.required) hasFatal = true;
      }
    } else if (c.cmd) {
      await new Promise((resolve) => {
        const p = spawn(c.cmd, c.args, { stdio: 'pipe' });
        let out = '';
        p.stdout.on('data', (d) => (out += d.toString()));
        p.stderr.on('data', (d) => (out += d.toString()));
        p.on('close', (code) => {
          if (code === 0) {
            console.log(`  [PASS] ${c.name}: ${out.trim()}`);
          } else {
            console.log(`  ${c.required ? '[FAIL]' : '[WARN]'} ${c.name}: Not available`);
            if (c.required) hasFatal = true;
          }
          resolve(null);
        });
        p.on('error', () => {
          console.log(`  ${c.required ? '[FAIL]' : '[WARN]'} ${c.name}: Command '${c.cmd}' not found`);
          if (c.required) hasFatal = true;
          resolve(null);
        });
      });
    }
  }

  console.log(hasFatal ? `\n[FAIL] Preflight detected missing essential dependencies.\n` : `\n[PASS] System diagnostics completed.\n`);
  process.exit(hasFatal ? 1 : 0);
}

const args = process.argv.slice(2);
if (args.length === 0 || args[0] === '--help' || args[0] === '-h' || args[0] === 'help') {
  showBanner();
  process.exit(0);
}

if (args[0] === 'version' || args[0] === '-v' || args[0] === '--version') {
  console.log(`openreview v${VERSION}`);
  process.exit(0);
}

if (args[0] === 'doctor') {
  await runDoctor();
  process.exit(0);
}

if (args[0] === 'skills') {
  const sub = args[1] || 'list';
  if (sub === 'install' || sub === 'add') {
    console.log('[OpenReview] Installing skills via skills.sh...');
    const runner = typeof process.versions.bun !== 'undefined' ? 'bunx' : 'npx';
    const child = spawn(runner, ['skills', 'add', 'imMamdouhaboammar/openreview'], { stdio: 'inherit' });
    child.on('close', (code) => process.exit(code ?? 0));
    return;
  } else {
    console.log(`\nOpenReview Bundled Skills:`);
    console.log(`  - openreview-orchestrator: Master intent router (REVIEW, FIX, INVESTIGATE, VERIFY_CONFIG)`);
    console.log(`  - deep-digger: Causal root cause investigation, ddmin token/line minimizer, flake oracle`);
    console.log(`  - openreview: Invariant auditor & review formatting standards\n`);
    console.log(`Install into your active agent:`);
    console.log(`  npx skills add imMamdouhaboammar/openreview\n`);
    process.exit(0);
  }
}

// Check native binary execution
const nativeBin = findBinary();
if (nativeBin) {
  const child = spawn(nativeBin, args, {
    stdio: 'inherit',
    env: process.env,
  });
  child.on('close', (code) => process.exit(code ?? 0));
} else {
  // Try running via cargo if source is available
  if (existsSync(join(rootDir, 'Cargo.toml'))) {
    console.log('[OpenReview] Native binary not found in release cache; invoking via cargo...');
    const child = spawn('cargo', ['run', '--release', '--', ...args], {
      cwd: rootDir,
      stdio: 'inherit',
      env: process.env,
    });
    child.on('close', (code) => process.exit(code ?? 0));
  } else {
    console.error(`[OpenReview] Error: Native binary not compiled and Cargo manifest not present.`);
    console.error(`To compile the native Rust microservice, run:`);
    console.error(`  cargo build --release`);
    console.error(`Or run via Docker:`);
    console.error(`  docker run --rm ghcr.io/immamdouhaboammar/openreview ${args.join(' ')}\n`);
    process.exit(1);
  }
}
