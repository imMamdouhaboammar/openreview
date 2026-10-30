#!/usr/bin/env bash
# Deep Digger Installer
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_NAME="deep-digger"

echo "[Deep Digger] Installing ${TARGET_NAME} (Catch That Bug)..."

install_to() {
  local target_dir="$1"
  local dest="${target_dir}/${TARGET_NAME}"
  if [ "$SCRIPT_DIR" = "$dest" ]; then
    echo "  [OK] Already in place -> $dest"
    return 0
  fi
  mkdir -p "$target_dir"
  rm -rf "$dest"
  cp -r "$SCRIPT_DIR" "$dest"
  echo "  [OK] Installed -> $dest"
}

# Claude Code
if [ -d "$HOME/.claude" ] || command -v claude >/dev/null 2>&1; then
  install_to "$HOME/.claude/skills"
fi

# Antigravity / Gemini CLI
GEMINI_HOME="${HOME}/.gemini"
if [ -d "${GEMINI_HOME}" ]; then
  install_to "${GEMINI_HOME}/config/skills"
fi

# Codex / OpenCode
if [ -d "$HOME/.codex" ]; then
  install_to "$HOME/.codex/skills"
fi

# Universal Agent Kernel (~/.agents/skills)
install_to "$HOME/.agents/skills"

# Optional local user bin symlink
if [ -d "$HOME/.local/bin" ]; then
  mkdir -p "$HOME/.local/bin"
  ln -sf "${SCRIPT_DIR}/bin/cli.js" "$HOME/.local/bin/deep-digger"
  echo "  [OK] Symlinked CLI to $HOME/.local/bin/deep-digger"
fi

echo ""
echo "[Deep Digger] Installation complete."
echo "Run 'deep-digger doctor' or 'npx deep-digger --help' to verify."
