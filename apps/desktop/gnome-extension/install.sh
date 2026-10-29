#!/usr/bin/env bash
# Installs the Slimy pointer bridge for the current user and enables it.
# On Wayland, GNOME Shell only discovers a new extension at login: log out and back in once.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
uuid="slimy-pointer@slimy.dev"
target="$HOME/.local/share/gnome-shell/extensions/$uuid"
mkdir -p "$target"
cp "$here/$uuid/metadata.json" "$here/$uuid/extension.js" "$target/"
gnome-extensions enable "$uuid" || echo "Enable it after the next login: gnome-extensions enable $uuid"
echo "Installed in $target"
