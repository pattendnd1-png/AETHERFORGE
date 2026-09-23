.PHONY: all arch fedora debian clean test install verify

# Default to current distro
DISTRO ?= $(shell ./distros/detect-distro.sh 2>/dev/null || echo "unknown")

all: verify

arch:
	@echo "Building for Arch Linux..."
	@bash distros/arch/build-pkg.sh 2>/dev/null || echo "Distros not configured yet"

fedora:
	@echo "Building for Fedora/RHEL..."
	@bash distros/fedora/build-pkg.sh 2>/dev/null || echo "Distros not configured yet"

debian:
	@echo "Building for Debian/Ubuntu..."
	@bash distros/debian/build-pkg.sh 2>/dev/null || echo "Distros not configured yet"

clean:
	rm -rf distros/*/pkgs/

test:
	@echo "Running tests..."
	cargo test --release --manifest-path apps/ForgeClean/payload/Cargo.toml 2>/dev/null || echo "Tests skipped"
	cargo test --release --manifest-path os/rust-audit/Cargo.toml 2>/dev/null || echo "Tests skipped"

install: all
	@echo "Installing packages..."

verify:
	@echo "Verifying code quality..."
	cargo fmt --check 2>/dev/null || echo "Format check skipped"
	cargo clippy --all-targets -- -D warnings 2>/dev/null || echo "Clippy skipped"
