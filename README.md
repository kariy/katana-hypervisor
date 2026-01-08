# Katana Hypervisor

A hypervisor for managing isolated [Katana](https://github.com/dojoengine/dojo) instances with QEMU/KVM virtualization and optional AMD SEV-SNP TEE support.

## What is Katana Hypervisor?

Katana Hypervisor lets you run multiple isolated Katana (Starknet sequencer) instances as virtual machines, each with dedicated resources and strong isolation guarantees. Think of it as a way to deploy and manage Katana instances with hardware-level security boundaries, perfect for production environments and confidential computing scenarios.

## Why Use Katana Hypervisor?

- **Production-Ready Isolation**: Each Katana instance runs in its own VM with hardware-level isolation via QEMU/KVM
- **Resource Control**: Set precise CPU, memory, and storage limits per instance
- **Confidential Computing**: Optional AMD SEV-SNP support for encrypted memory and attestation
- **Simple Management**: Single CLI tool to create, manage, and monitor all your Katana instances
- **Zero Configuration**: Pre-built VM components included—just install and run
- **State Persistence**: Instance configurations survive hypervisor restarts

## Features

### Core VM Management

- ✅ **Instance Lifecycle**: Create, start, stop, delete instances with one command
- ✅ **Automatic Startup**: Instances start immediately after creation
- ✅ **Resource Limits**: Configure vCPUs, memory, and storage per instance
- ✅ **Port Management**: Automatic port allocation (starting from 5050)
- ✅ **Log Streaming**: View and follow serial console logs
- ✅ **Status Monitoring**: Real-time stats via QMP (CPU, memory, uptime)
- ✅ **State Persistence**: SQLite database tracks all instances

### TEE Support (Optional)

- ✅ **AMD SEV-SNP**: Enable confidential computing with encrypted memory
- ✅ **Attestation Ready**: Automatic TEE configuration when `--tee` flag is used
- ✅ **OVMF UEFI**: Pre-configured firmware for secure boot

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Katana Hypervisor CLI                      │
│  (create, start, stop, list, logs, stats, delete)      │
├─────────────────────────────────────────────────────────┤
│  State DB  │  Port Allocator  │  Storage Manager       │
└────────────┴──────────────────┴─────────────────────────┘
                        │
                        ▼
          ┌─────────────────────────────┐
          │      QEMU/KVM Manager       │
          │   (QMP for monitoring)      │
          └─────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
   ┌────────┐      ┌────────┐      ┌────────┐
   │ VM 1   │      │ VM 2   │      │ VM N   │
   │ Katana │      │ Katana │      │ Katana │
   │ :5050  │      │ :5051  │      │ :505N  │
   └────────┘      └────────┘      └────────┘
```

Each VM gets:
- Dedicated vCPUs and memory
- Isolated storage directory
- Unique RPC port
- Serial console for logging
- QMP socket for management

## Prerequisites

### Required

- **Linux**: Ubuntu 24.04+, RHEL 9+, Fedora 39+
- **KVM Support**: Check with `lsmod | grep kvm`
- **QEMU**: Version 7.0+ (`qemu-system-x86_64`)
- **Rust**: Version 1.75+ (for building from source)

### Optional (for TEE mode)

- **AMD EPYC**: 3rd gen (Milan) or newer
- **SEV-SNP**: Enabled in BIOS
- **Device**: `/dev/sev-guest` available

### Install Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y qemu-system-x86 qemu-kvm
sudo usermod -aG kvm $USER
# Log out and back in for group changes to take effect
```

**RHEL/Fedora:**
```bash
sudo dnf install -y qemu-kvm qemu-system-x86
sudo usermod -aG kvm $USER
# Log out and back in for group changes to take effect
```

**Verify KVM access:**
```bash
ls -la /dev/kvm
# Should show: crw-rw---- 1 root kvm
```

## Installation

### Build from Source

```bash
# Clone the repository
git clone https://github.com/kariy/katana-hypervisor.git
cd katana-hypervisor

# Build release binary
cargo build --release

# Binary available at: target/release/katana-hypervisor
```

### Install Globally (Optional)

```bash
cargo install --path .
# Now available as: katana-hypervisor
```

**Note**: Boot components (kernel, initrd, OVMF) are included in the repository. No separate build step needed!

## Quick Start

### Create Your First Instance

```bash
# Create and automatically start an instance with default settings
# (4 vCPUs, 4GB memory, 10GB storage, auto-assigned port)
katana-hypervisor create dev1

# Instance is now running! Wait ~5 seconds for Katana to initialize
sleep 5

# Test the RPC endpoint
curl http://localhost:5050/
```

### Create with Custom Resources

```bash
# Create instance with specific resources
katana-hypervisor create prod1 \
  --vcpus 8 \
  --memory 16G \
  --storage 50G \
  --port 5100

# Create development instance with minimal resources
katana-hypervisor create test1 \
  --vcpus 2 \
  --memory 2G \
  --storage 5G \
  --dev
```

### Create with TEE Support

```bash
# Requires AMD SEV-SNP capable hardware
katana-hypervisor create secure1 \
  --vcpus 4 \
  --memory 8G \
  --tee

# Katana will automatically run with TEE provider enabled
```

## Command Reference

### Instance Management

#### create
Create and start a new Katana instance.

```bash
katana-hypervisor create <name> [OPTIONS]

Options:
  --vcpus <COUNT>       Number of virtual CPUs [default: 4]
  --memory <SIZE>       Memory limit (e.g., 4G, 512M) [default: 4G]
  --storage <SIZE>      Storage quota (e.g., 10G, 50G) [default: 10G]
  --port <PORT>         RPC port [default: auto-assign from 5050]
  --dev                 Enable dev mode (no fees, pre-funded accounts)
  --tee                 Enable AMD SEV-SNP TEE mode
  --vcpu-type <TYPE>    CPU type for TEE [default: EPYC-v4]

Examples:
  katana-hypervisor create dev1
  katana-hypervisor create prod1 --vcpus 8 --memory 16G --storage 100G
  katana-hypervisor create secure1 --tee
```

#### start
Start a stopped instance.

```bash
katana-hypervisor start <name>

Example:
  katana-hypervisor start dev1
```

#### stop
Gracefully stop a running instance.

```bash
katana-hypervisor stop <name>

Example:
  katana-hypervisor stop dev1
```

#### delete
Delete an instance and its data.

```bash
katana-hypervisor delete <name> [OPTIONS]

Options:
  --force    Force deletion even if running

Examples:
  katana-hypervisor delete dev1
  katana-hypervisor delete dev1 --force
```

#### list
List all instances with their status.

```bash
katana-hypervisor list

Example output:
NAME       STATUS    VCPUS  TEE  MEMORY   PORT   PID
-----------------------------------------------------
dev1       running   4      no   3814M    5050   12345
prod1      stopped   8      no   16384M   5100   -
secure1    running   4      yes  7629M    5101   67890
```

### Monitoring

#### logs
View instance logs from serial console.

```bash
katana-hypervisor logs <name> [OPTIONS]

Options:
  -f, --follow         Follow log output (like tail -f)
  -n, --tail <LINES>   Number of lines to show [default: 100]

Examples:
  katana-hypervisor logs dev1
  katana-hypervisor logs dev1 -f
  katana-hypervisor logs dev1 -n 50
```

#### stats
View instance resource statistics.

```bash
katana-hypervisor stats <name> [OPTIONS]

Options:
  --watch              Continuous monitoring mode
  --interval <SEC>     Update interval for watch mode [default: 2]

Examples:
  katana-hypervisor stats dev1
  katana-hypervisor stats dev1 --watch
  katana-hypervisor stats dev1 --watch --interval 5

Output includes:
  - VM status (running/paused)
  - Process uptime
  - CPU count and thread IDs
  - Memory allocation
  - RPC endpoint
```

## Common Workflows

### Development Workflow

```bash
# Create development instance
katana-hypervisor create dev --dev --vcpus 2 --memory 2G

# View logs in real-time
katana-hypervisor logs dev -f

# In another terminal, interact with Katana
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# When done, clean up
katana-hypervisor delete dev --force
```

### Production Deployment

```bash
# Create production instance with generous resources
katana-hypervisor create prod \
  --vcpus 8 \
  --memory 16G \
  --storage 100G \
  --port 5050

# Monitor continuously
katana-hypervisor stats prod --watch

# Check logs periodically
katana-hypervisor logs prod -n 50

# Graceful shutdown when needed
katana-hypervisor stop prod
```

### Multi-Instance Setup

```bash
# Create multiple instances for different purposes
katana-hypervisor create mainnet --vcpus 8 --memory 16G --port 5050
katana-hypervisor create testnet --vcpus 4 --memory 8G --port 5051
katana-hypervisor create dev --vcpus 2 --memory 2G --port 5052 --dev

# List all instances
katana-hypervisor list

# Monitor specific instance
katana-hypervisor stats mainnet --watch
```

### TEE/Confidential Computing

```bash
# Create confidential instance (requires SEV-SNP hardware)
katana-hypervisor create secure \
  --vcpus 4 \
  --memory 8G \
  --tee \
  --vcpu-type EPYC-v4

# Verify TEE mode is enabled
katana-hypervisor list
# Look for "TEE" column showing "yes"

# The Katana instance automatically runs with:
# --tee.provider sev-snp
```

## Testing Your Instance

### Health Check

```bash
# Wait for Katana to initialize
sleep 5

# Simple health check
curl http://localhost:5050/

# Expected: HTTP 200 OK with HTML page
```

### RPC Methods

```bash
# Get chain ID
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_chainId","params":[],"id":1}'

# Get block number
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_blockNumber","params":[],"id":1}'

# Get syncing status
curl -X POST http://localhost:5050 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"starknet_syncing","params":[],"id":1}'
```

## Configuration

### Storage Locations

**State Database:**
```
~/.local/share/hypervisor/state.db
```
Contains instance configurations, port allocations, and boot component metadata.

**Instance Data:**
```
~/.local/share/hypervisor/instances/<instance-id>/
├── data/           # Katana database
├── serial.log      # Console output
└── qmp.sock        # QEMU control socket
```

**Boot Components:**
```
katana-hypervisor/boot-components/
├── vmlinuz         # Linux kernel (~15MB)
├── initrd.img      # Initrd with Katana binary (~21MB)
└── ovmf.fd         # UEFI firmware (~3.5MB)
```

### Environment Variables

Override state directory:
```bash
katana-hypervisor --state-dir /custom/path create dev1
```

Enable verbose logging:
```bash
katana-hypervisor -v create dev1
```

## Troubleshooting

### "Permission denied" when accessing /dev/kvm

**Problem:** User not in `kvm` group.

**Solution:**
```bash
sudo usermod -aG kvm $USER
# Log out and log back in
```

### "Port already in use"

**Problem:** Another instance or service is using the port.

**Solution:**
```bash
# Let the hypervisor auto-assign a port
katana-hypervisor create dev1

# Or specify a different port
katana-hypervisor create dev1 --port 5100
```

### Instance won't start

**Problem:** Boot components missing or QEMU issue.

**Solution:**
```bash
# Check logs for error details
katana-hypervisor logs <instance-name>

# Verify QEMU is installed
which qemu-system-x86_64

# Check KVM is loaded
lsmod | grep kvm

# Try deleting and recreating
katana-hypervisor delete <instance-name> --force
katana-hypervisor create <instance-name>
```

### Katana not responding

**Problem:** Instance may still be booting.

**Solution:**
```bash
# Wait ~5 seconds for boot and initialization
sleep 5

# Check logs to see boot progress
katana-hypervisor logs <instance-name>

# Look for "RPC server started" message

# Monitor stats
katana-hypervisor stats <instance-name>
```

### Instance stuck in "starting" state

**Problem:** VM failed to start but state wasn't updated.

**Solution:**
```bash
# Force delete and recreate
katana-hypervisor delete <instance-name> --force
katana-hypervisor create <instance-name>
```

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Boot time | ~5 seconds (to RPC ready) |
| Memory overhead | ~50MB per VM (QEMU + kernel) |
| Disk overhead | ~40MB (shared boot components) + instance data |
| CPU overhead | Minimal with KVM acceleration |

## Security Considerations

- **VM Isolation**: Each instance runs in a separate QEMU process with hardware virtualization
- **Network**: Instances bind to 127.0.0.1 (localhost) by default
- **Filesystem**: Instance directories have 700 permissions (owner-only access)
- **State DB**: Contains no secrets, only configuration metadata
- **TEE Mode**: Provides memory encryption and attestation (SEV-SNP)

## Comparison with Other Deployment Methods

| Feature | Katana Hypervisor | Docker | Bare Metal |
|---------|------------------|--------|------------|
| Isolation | Hardware (VM) | Process/cgroup | None |
| Resource Limits | Hard (VM) | Soft (cgroup) | None |
| TEE Support | ✅ SEV-SNP | ❌ | ✅ |
| Boot Time | ~5s | ~1s | N/A |
| Memory Overhead | ~50MB/instance | ~10MB/instance | 0 |
| Multi-Instance | Easy | Easy | Manual |
| Best For | Production, TEE | Development | Single instance |

## Development

### Project Structure

```
katana-hypervisor/
├── src/
│   ├── cli/          # CLI command implementations
│   │   ├── create.rs
│   │   ├── start.rs
│   │   ├── stop.rs
│   │   ├── delete.rs
│   │   ├── list.rs
│   │   ├── logs.rs
│   │   └── stats.rs
│   ├── instance/     # Instance management & storage
│   ├── port/         # Port allocation logic
│   ├── qemu/         # QEMU/KVM integration
│   │   ├── vm.rs     # VM lifecycle
│   │   ├── qmp.rs    # QMP client
│   │   └── config.rs # QEMU configuration
│   ├── state/        # SQLite database
│   └── tee/          # SEV-SNP configuration
├── boot-components/  # VM boot files (kernel, initrd, OVMF)
└── tests/           # Integration tests
```

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

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes with tests
4. Ensure all tests pass: `cargo test`
5. Commit your changes: `git commit -m "Add my feature"`
6. Push to the branch: `git push origin feature/my-feature`
7. Submit a pull request

### Areas for Contribution

- Storage quota enforcement (filesystem quotas on Linux)
- JSON/YAML output formats for list command
- Additional QMP monitoring metrics
- Windows/macOS support (non-KVM backends)
- Instance templates/presets
- VM snapshot/restore functionality
- Live migration support
- Prometheus metrics exporter

## License

MIT License - see LICENSE file for details

## Related Projects

- [Katana](https://github.com/dojoengine/dojo) - Starknet sequencer
- [Dojo](https://github.com/dojoengine/dojo) - Provable game engine
- [QEMU](https://www.qemu.org/) - Machine emulator and virtualizer
- [AMD SEV-SNP](https://www.amd.com/en/processors/amd-secure-encrypted-virtualization) - Secure Encrypted Virtualization

## Support

- **Issues**: [GitHub Issues](https://github.com/kariy/katana-hypervisor/issues)
- **Discussions**: [GitHub Discussions](https://github.com/kariy/katana-hypervisor/discussions)
- **Dojo Community**: [Dojo Discord](https://discord.gg/dojoengine)

## Acknowledgments

Built for the Dojo/Katana ecosystem to enable production deployments with hardware-level isolation and confidential computing support.
