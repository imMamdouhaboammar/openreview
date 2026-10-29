#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_NAME="openreview-orchestrator"

echo "Installing ${TARGET_NAME} across AI agent environments..."

# Default installation into universal agent skills directory
DEST_DIR="${SKILLS_DIR:-$HOME/.agents/skills/${TARGET_NAME}}"
mkdir -p "$DEST_DIR"
cp -r "$SCRIPT_DIR/"* "$DEST_DIR/"
echo "  Installed for Universal Agent Kernel -> $DEST_DIR"

# Support custom target directory via environment variable
if [ -n "$CUSTOM_SKILLS_DIR" ]; then
  mkdir -p "$CUSTOM_SKILLS_DIR/${TARGET_NAME}"
  cp -r "$SCRIPT_DIR/"* "$CUSTOM_SKILLS_DIR/${TARGET_NAME}/"
  echo "  Installed to custom target -> $CUSTOM_SKILLS_DIR/${TARGET_NAME}"
fi

echo ""
echo "Installation complete!"
