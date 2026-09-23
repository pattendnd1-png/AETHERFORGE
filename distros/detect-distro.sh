#!/bin/bash
# AetherForge Distribution Detector
# Returns: arch, fedora, debian, or unknown

get_distro() {
    if [ -f /etc/arch-release ]; then
        echo "arch"
        return 0
    elif [ -f /etc/fedora-release ]; then
        echo "fedora"
        return 0
    elif [ -f /etc/redhat-release ]; then
        echo "rhel"
        return 0
    elif [ -f /etc/debian_version ]; then
        echo "debian"
        return 0
    elif grep -qi "ubuntu" /etc/os-release 2>/dev/null; then
        echo "ubuntu"
        return 0
    else
        echo "unknown"
        return 1
    fi
}

# Export for other scripts
export AETHER_DISTRO=$(get_distro)
echo "Detected: $AETHER_DISTRO"
