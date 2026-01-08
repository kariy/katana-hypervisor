# Boot Components

This directory contains prebuilt VM boot components used by **all** katana instances.

All instances share the same kernel, initrd, and firmware - there's no per-instance customization needed.

## Files

- **vmlinuz** - Linux kernel (~15MB)
- **initrd.img** - Initrd with katana binary embedded (~21MB)
- **ovmf.fd** - UEFI firmware for SEV-SNP support (~3.5MB)

These files must exist before creating any instances. The hypervisor validates their presence during instance creation.

## Building Boot Components

⚠️ **Required before first use**: These files are NOT checked into git due to size. You must build them:

```bash
# Build boot components from katana repository
cd /home/ubuntu/katana
make build-tee

# Copy to hypervisor boot-components directory
cp tee/build/vmlinuz /home/ubuntu/katana-hypervisor/boot-components/
cp tee/build/initrd.img /home/ubuntu/katana-hypervisor/boot-components/
cp tee/build/ovmf.fd /home/ubuntu/katana-hypervisor/boot-components/
```

### Reproducibility

All components are built with `SOURCE_DATE_EPOCH` to ensure bit-for-bit reproducibility. This is **critical** for SEV-SNP attestation where the launch measurement must be deterministic across builds.

## Error Messages

If you see errors like:
```
Kernel not found at: boot-components/vmlinuz
Boot components must be built first. See boot-components/README.md
```

This means you need to build the boot components using the instructions above.

## Distribution

For production deployments:
- Build components once using reproducible builds
- Include in installation package or distribution
- All users/instances share the same prebuilt components
- Update components when katana version changes
