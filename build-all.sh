#!/bin/bash
set -euo pipefail

echo "=========================================================="
echo "  AETHER-OS Multi-Distro Package Builder"
echo "=========================================================="
echo ""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AETHER_ROOT="${AETHER_ROOT:-/home/benji/AETHER-OS}"

# Detect current distro
source "$AETHER_ROOT/distros/detect-distro.sh"
DISTRO="$AETHER_DISTRO"

echo "Running on: $DISTRO"
echo ""

case "$DISTRO" in
    arch)
        echo "🐧 Building for Arch Linux..."
        bash "$AETHER_ROOT/distros/arch/build-pkg.sh"
        ;;
    fedora|rhel)
        echo "🔴 Building for Fedora/RHEL..."
        bash "$AETHER_ROOT/distros/fedora/build-pkg.sh"
        ;;
    debian|ubuntu)
        echo "🟢 Building for Debian/Ubuntu..."
        bash "$AETHER_ROOT/distros/debian/build-pkg.sh"
        ;;
    *)
        echo "⚠️  Unknown distro. Building locally only."
        echo "    Run manually from:"
        echo "      - distros/arch/build-pkg.sh"
        echo "      - distros/fedora/build-pkg.sh"
        echo "      - distros/debian/build-pkg.sh"
        ;;
esac

echo ""
echo "=========================================================="
echo "  BUILD COMPLETE"
echo "=========================================================="
echo ""
echo "Package locations:"
echo "  Arch:   $AETHER_ROOT/distros/arch/pkgs/repo/"
echo "  Fedora: $AETHER_ROOT/distros/fedora/pkgs/RPMS/x86_64/"
echo "  Debian: $AETHER_ROOT/distros/debian/pkgs/"
