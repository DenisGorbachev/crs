#!/usr/bin/env bash

set -euo pipefail

fd --type f --extension sh --extension bash --extension zsh . . .mise/tasks .agents/skills .repoconf/hooks --exec-batch shellcheck --shell bash
for file in ./*; do
  case "$file" in
    *.sh)
      if [ -f "$file" ]; then
        shellcheck --shell bash "$file"
      fi
      ;;
  esac
done
