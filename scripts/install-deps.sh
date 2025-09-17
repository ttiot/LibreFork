#!/usr/bin/env bash
set -euo pipefail

echo "Installing native dependencies for GTK4 + libadwaita..."

if command -v apt-get >/dev/null 2>&1; then
  sudo apt-get update
  sudo apt-get install -y build-essential pkg-config \
    libgtk-4-dev libadwaita-1-dev \
    gobject-introspection libgirepository1.0-dev
elif command -v dnf >/dev/null 2>&1; then
  sudo dnf install -y gcc gcc-c++ make pkgconfig \
    gtk4-devel libadwaita-devel gobject-introspection-devel
elif command -v pacman >/dev/null 2>&1; then
  sudo pacman -S --needed --noconfirm base-devel pkgconf \
    gtk4 libadwaita gobject-introspection
elif command -v brew >/dev/null 2>&1; then
  brew install gtk4 libadwaita gobject-introspection
else
  cat <<EOF
Unsupported distribution. Please install development packages for:
  - GTK4 (headers and pkg-config files)
  - libadwaita (headers and pkg-config files)
Also ensure a working C toolchain (gcc/clang) and pkg-config.
EOF
fi

# Quick sanity check (optional)
echo "Verifying pkg-config availability of required modules..."
if ! pkg-config --exists glib-2.0 gobject-introspection-1.0 gtk4 libadwaita-1; then
  echo "Some pkg-config modules are still missing. Ensure PKG_CONFIG_PATH is set if you installed to a non-standard prefix." >&2
  echo "Example: export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig:/opt/homebrew/lib/pkgconfig" >&2
  exit 1
fi

echo "Done. You can now build with: go build ./cmd/librefork"
