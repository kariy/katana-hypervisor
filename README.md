# Katana Hypervisor

A hypervisor for managing isolated [Katana](https://github.com/dojoengine/katana) instances as virtual machines using QEMU/KVM with optional AMD SEV-SNP support.

## Overview

Run multiple Katana instances with hardware-level isolation. Each instance operates in a dedicated VM with configurable resources.

**Core Capabilities:**
- Create, start, stop, and delete isolated Katana instances
- Configure vCPUs, memory, and storage per instance
- Monitor resource usage and view logs
- Optional AMD SEV-SNP for encrypted memory (soon)

## Prerequisites

**Required:**
- Linux (Ubuntu 24.04+, RHEL 9+, Fedora 39+)
- QEMU 7.0+ with KVM support
- Rust 1.75+ (for building)

**Optional (for TEE):**
- AMD EPYC 3rd gen+ with SEV-SNP enabled

**Install dependencies:**
```bash
# Ubuntu/Debian
sudo apt install -y qemu-system-x86 qemu-kvm
sudo usermod -aG kvm $USER

# RHEL/Fedora
sudo dnf install -y qemu-kvm qemu-system-x86
sudo usermod -aG kvm $USER
```

Log out and back in for group changes to take effect.

## Troubleshooting

### "Permission denied" when accessing /dev/kvm

**Problem:** User not in `kvm` group.

**Solution:**
```bash
sudo usermod -aG kvm $USER
# Log out and log back in
```

## Development

### Running Tests

```bash
# All tests
cargo test

# Integration tests only
cargo test --test '*'

# With output
cargo test -- --nocapture

# Specific test
cargo test test_port_allocation
```

### Building Boot Components

Boot components are pre-built and included in the repository. To rebuild them:

```bash
# From the katana repository root
cd /path/to/katana
make build-tee

# Copy artifacts to hypervisor repository
cp tee-build-output/vmlinuz /path/to/katana-hypervisor/boot-components/
cp tee-build-output/initrd.img /path/to/katana-hypervisor/boot-components/
cp tee-build-output/ovmf.fd /path/to/katana-hypervisor/boot-components/
```
