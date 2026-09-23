#!/bin/bash
set -euo pipefail

AETHER_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  build     - Build for current distro"
    echo "  test      - Run all tests"
    echo "  verify    - Check code quality"
    echo "  clean     - Remove build artifacts"
    echo "  help      - Show this help"
    echo ""
    echo "Distro-specific:"
    echo "  arch      - Build for Arch Linux"
    echo "  fedora    - Build for Fedora/RHEL"
    echo "  debian    - Build for Debian/Ubuntu"
}

case "${1:-build}" in
    build)
        source "$AETHER_ROOT/distros/detect-distro.sh"
        case "$AETHER_DISTRO" in
            arch) bash "$AETHER_ROOT/distros/arch/build-pkg.sh" ;;
            fedora|rhel) bash "$AETHER_ROOT/distros/fedora/build-pkg.sh" ;;
            debian|ubuntu) bash "$AETHER_ROOT/distros/debian/build-pkg.sh" ;;
            *) echo "Unknown distro. Use: arch, fedora, or debian" ;;
        esac
        ;;
    test)
        cd "$AETHER_ROOT/apps/ForgeClean/payload" && cargo test --release
        cd "$AETHER_ROOT/os/rust-audit" && cargo test --release
        ;;
    verify)
        cargo fmt --check
        cargo clippy --all-targets -- -D warnings
        ;;
    clean)
        find . -type d -name target -exec rm -rf {} + 2>/dev/null || true
        ;;
    help|--help|-h)
        usage
        ;;
    arch)
        bash "$AETHER_ROOT/distros/arch/build-pkg.sh"
        ;;
    fedora)
        bash "$AETHER_ROOT/distros/fedora/build-pkg.sh"
        ;;
    debian)
        bash "$AETHER_ROOT/distros/debian/build-pkg.sh"
        ;;
    *)
        echo "Unknown command: $1"
        usage
        exit 1
        ;;
esac
