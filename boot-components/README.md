# Boot Components

This directory contains prebuilt VM boot components used by **all** katana instances.

All instances share the same kernel, initrd, and firmware - there's no per-instance customization needed.

## Files (Included in Repository)

✅ **vmlinuz** - Linux kernel (~15MB)
✅ **initrd.img** - Initrd with katana binary embedded (~21MB)
✅ **ovmf.fd** - UEFI firmware for SEV-SNP support (~3.5MB)

These files are **included in the git repository** and ready to use immediately after cloning. No build step required!

## Version Information

These boot components are built from the Katana repository using reproducible builds with `SOURCE_DATE_EPOCH` to ensure bit-for-bit determinism.

**Build Details:**
- Katana version: 1.7.0
- Build date: 2026-01-08
- SHA-384 hash: `11897dac7266a79d410c7d019117906bab65ef08096b05d7ea8b94d2db333b4d22362d39ad6313c42955c8dc5fbc67db`

### Reproducibility

All components are built with `SOURCE_DATE_EPOCH` for reproducible builds. This is **critical** for SEV-SNP attestation where the launch measurement must be deterministic.

## Rebuilding Boot Components (Optional)

If you need to rebuild with a different Katana version:

```bash
# Build boot components from katana repository
cd /path/to/katana
make build-tee

# Copy to hypervisor boot-components directory
cp output/vmlinuz boot-components/
cp output/initrd.img boot-components/
cp output/ovmf.fd boot-components/
```

## Usage

The hypervisor automatically uses these components for all instances. The boot component paths are validated during instance creation:

```bash
# This will automatically use boot-components/vmlinuz, initrd.img, and ovmf.fd
katana-hypervisor create my-instance
```

## Distribution

For production deployments:
- Boot components are included in the repository
- All users/instances share the same prebuilt components
- Update components by rebuilding when katana version changes
- Measurements will change if you rebuild with different source
