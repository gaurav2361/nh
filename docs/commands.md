# Commands Reference

This document lists common `nh` commands and their usage.

## Clean

The `clean` command helps reclaim space by removing old Nix generations and system caches.

### Nix Cleanup
Clean all profiles and garbage collect the Nix store:
```bash
nh clean all
```

Options:
- `--keep 5`: Keep the last 5 generations (default: 1)
- `--keep-since 30d`: Keep generations newer than 30 days
- `--dry`: Show what would be removed without making changes

### Darwin Cleanup (macOS only)
Perform deep system cleaning on macOS:
```bash
nh clean darwin
```

This includes:
- System and User caches
- Developer tools (NPM, Cargo, Docker)
- Browser caches (Chrome, Safari, Brave)
- Build artifacts in project folders
- System optimizations (DNS flush, etc.)

**Dry run (recommended):**
```bash
nh clean darwin --dry
```

### Selective Darwin Cleaning
You can specify categories to clean:
```bash
# Clean only dev and apps
nh clean darwin --categories dev,apps
```

Categories: `system`, `user`, `dev`, `apps`, `browsers`, `optimize`, `purge`, `nix`.

## Search

Search for Nix packages:
```bash
nh search <query>
```

## Home / NixOS Management

Commands for switching configurations:
```bash
# Apply Home Manager configuration
nh home switch

# Apply NixOS configuration
nh os switch
```
