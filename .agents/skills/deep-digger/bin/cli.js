#!/usr/bin/env node
/** @license MIT · Deep Digger */
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const rootDir = join(__dirname, '..');
const pythonScript = existsSync(join(__dirname, 'scripts', 'deep_digger.py'))
  ? join(__dirname, 'scripts', 'deep_digger.py')
  : join(rootDir, 'scripts', 'deep_digger.py');

const VERSION = '1.1.0';

function showBanner() {
  console.log(`
Deep Digger v${VERSION}: Catch That Bug
Evidence-Gated Root Cause Investigation & Controlled Falsification for AI Agents

Usage:
  deep-digger <command> [options]

Commands:
  init <folder>                 Initialize a new investigation case ledger
  hypothesis <folder> add ...   Register a falsifiable causal hypothesis
  evidence <folder> add ...     Register tamper-evident artifact evidence
  experiment <folder> add ...   Record controlled test receipts (baseline, intervention, reversal, control)
  trace <folder> --file <f> ... Ingest OpenTelemetry, HAR, or JSON trace into evidence ledger
  link <folder> --parent <p>    Model directed causal relationships (DAG) with cycle prevention
  flake <folder> --oracle <cmd> Statistical repetition oracle to compute empirical P(FAIL)
  bisect <folder> --oracle ...  Automated git bisect runner recorded into evidence ledger
  repro <folder> --template ... Generate isolated test harness fixture (python, node, bash)
  minimize <input> --oracle ... Delta-debugging minimizer for failing inputs (lines or tokens)
  report <folder> [--html <f>]  Generate causal claim ladder report (text, json, or html)
  view <folder> [--no-color]    Render interactive ANSI terminal dashboard for case ledger
  doctor                        Verify environment dependencies (Python, Cargo, Bun)
  version                       Show current version

Examples:
  deep-digger init cases/auth-bug
  deep-digger hypothesis cases/auth-bug add H-token-expiry "Token expires prematurely on leap seconds"
  deep-digger view cases/auth-bug
  deep-digger report cases/auth-bug --html cases/auth-bug/report.html
  deep-digger doctor

Universal Agent Integration:
  Skills Registry:  npx skills add imMamdouhaboammar/deep-digger
  Direct Install:   git clone https://github.com/imMamdouhaboammar/deep-digger.git && ./deep-digger/install.sh
`);
}

async function runDoctor() {
  console.log(`\n[Deep Digger] Running System Preflight Verification...\n`);
  const checks = [
    { name: 'Node / Bun Runtime', check: () => process.version },
    {
      name: 'Python 3 Standard Library',
      cmd: 'python3',
      args: ['--version'],
      required: true,
    },
    {
      name: 'Rust / Cargo (Optional for ultra-fast minimizer)',
      cmd: 'cargo',
      args: ['--version'],
      required: false,
    },
    {
      name: 'Deep Digger Python Core',
      check: () => (existsSync(pythonScript) ? 'Present (scripts/deep_digger.py)' : null),
      required: true,
    },
    {
      name: 'Deep Digger Skill Definition',
      check: () => {
        if (existsSync(join(__dirname, 'SKILL.md'))) return 'Valid (SKILL.md)';
        if (existsSync(join(rootDir, 'SKILL.md'))) return 'Valid (SKILL.md)';
        return null;
      },
      required: true,
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

  console.log(hasFatal ? `\n[FAIL] Preflight detected missing required dependencies.\n` : `\n[PASS] All essential components verified and ready.\n`);
  process.exit(hasFatal ? 1 : 0);
}

const args = process.argv.slice(2);
if (args.length === 0 || args[0] === '--help' || args[0] === '-h' || args[0] === 'help') {
  showBanner();
  process.exit(0);
}

if (args[0] === 'version' || args[0] === '-v' || args[0] === '--version') {
  console.log(`deep-digger v${VERSION}`);
  process.exit(0);
}

if (args[0] === 'doctor') {
  await runDoctor();
}

// Forward all commands to scripts/deep_digger.py
const child = spawn('python3', [pythonScript, ...args], {
  stdio: 'inherit',
  env: process.env,
});

child.on('close', (code) => {
  process.exit(code ?? 0);
});
