.PHONY: all arch fedora debian clean test install verify

# Default to current distro
DISTRO ?= $(shell ./distros/detect-distro.sh 2>/dev/null || echo "unknown")

all: $(DISTRO)

arch:
	@echo "Building for Arch Linux..."
	@bash distros/arch/build-pkg.sh

fedora:
	@echo "Building for Fedora/RHEL..."
	@bash distros/fedora/build-pkg.sh

debian:
	@echo "Building for Debian/Ubuntu..."
	@bash distros/debian/build-pkg.sh

clean:
	rm -rf distros/*/pkgs/*/src distros/*/pkgs/*/pkg
	rm -rf distros/fedora/pkgs/BUILD distros/fedora/pkgs/RPMS
	find . -name "*.orig" -delete

test:
	@echo "Running tests for all projects..."
	cargo test --release --manifest-path apps/ForgeClean/payload/Cargo.toml
	cargo test --release --manifest-path os/rust-audit/Cargo.toml

install: all
	@echo "Installing packages for $(DISTRO)..."

verify:
	@echo "Verifying builds..."
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

docs:
	@echo "Building documentation..."
