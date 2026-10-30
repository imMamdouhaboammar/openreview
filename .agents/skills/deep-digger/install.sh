#!/usr/bin/env bash
# Deep Digger Installer
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_NAME="deep-digger"

echo "[Deep Digger] Installing ${TARGET_NAME} (Catch That Bug)..."

# Claude Code
if [ -d "$HOME/.claude" ] || command -v claude >/dev/null 2>&1; then
  mkdir -p "$HOME/.claude/skills"
  rm -rf "$HOME/.claude/skills/${TARGET_NAME}"
  cp -r "$SCRIPT_DIR" "$HOME/.claude/skills/${TARGET_NAME}"
  echo "  [OK] Installed for Claude Code -> $HOME/.claude/skills/${TARGET_NAME}"
fi

# Antigravity / Gemini CLI
GEMINI_HOME="${HOME}/.gemini"
if [ -d "${GEMINI_HOME}" ]; then
  DEST_DIR="${GEMINI_HOME}/config/skills"
  mkdir -p "${DEST_DIR}"
  rm -rf "${DEST_DIR}/${TARGET_NAME}"
  cp -r "$SCRIPT_DIR" "${DEST_DIR}/${TARGET_NAME}"
  echo "  [OK] Installed for Antigravity / Gemini CLI -> ${DEST_DIR}/${TARGET_NAME}"
fi

# Codex / OpenCode
if [ -d "$HOME/.codex" ]; then
  mkdir -p "$HOME/.codex/skills"
  rm -rf "$HOME/.codex/skills/${TARGET_NAME}"
  cp -r "$SCRIPT_DIR" "$HOME/.codex/skills/${TARGET_NAME}"
  echo "  [OK] Installed for Codex / OpenCode -> $HOME/.codex/skills/${TARGET_NAME}"
fi

# Universal Agent Kernel (~/.agents/skills)
mkdir -p "$HOME/.agents/skills"
rm -rf "$HOME/.agents/skills/${TARGET_NAME}"
cp -r "$SCRIPT_DIR" "$HOME/.agents/skills/${TARGET_NAME}"
echo "  [OK] Installed for Universal Agent Kernel -> $HOME/.agents/skills/${TARGET_NAME}"

# Optional local user bin symlink
if [ -d "$HOME/.local/bin" ]; then
  mkdir -p "$HOME/.local/bin"
  ln -sf "${SCRIPT_DIR}/bin/cli.js" "$HOME/.local/bin/deep-digger"
  echo "  [OK] Symlinked CLI to $HOME/.local/bin/deep-digger"
fi

echo ""
echo "[Deep Digger] Installation complete."
echo "Run 'deep-digger doctor' or 'npx deep-digger --help' to verify."
