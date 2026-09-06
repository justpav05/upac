<h1 align="center">✅ TODO</h1> 

Near-term, concrete items. See `ROADMAP.md` for the bigger picture.

## upac-cli

- `user/upac-cli/data/` (`.desktop`, `upac-mime.xml`, `.policy`) reference `Icon=upac`/`icon_name=upac`,
  but there's no actual icon asset (SVG/PNG) yet, and no install step wiring it into
  `/usr/share/icons/hicolor/...`. Needs real artwork before packaging.

## upac-lib

Test-coverage pass in progress, going file by file through the non-command core first
(`errors.rs`/`lock.rs`/`search.rs`/`fs.rs`/`orchestrator/*`/`database/*` done), commands
(`mutated`/`unmutated`) last. Remaining core files not yet visited: `deploy/{error,retention,mod}.rs`
(`esp.rs` skipped — real mount), `scripts/{error,file,load,pipeline,primitive}.rs`,
`plugin/decoder/{error,unpack,mod}.rs`, `plugin/boot/{error,manifest,mod}.rs`,
`composefs/{diff,error,mod}.rs`, `config/mod.rs`, `boot/{error,mod}.rs`.

**UKI A/B boot (`upac-from.efi`/`upac-to.efi`) confirm-boot service not designed yet**: after a
successful boot, something needs to confirm once, swap `to`↔`from`, and set the normal persistent
boot order. Nothing calls `Booter::confirm_boot` anywhere yet; this belongs to a not-yet-designed
"confirm boot" systemd service, not genesis or the ordinary install/update pipeline.

**grub genesis support still not handled**: unlike systemd-boot/rEFInd (binary-copy via
`esp_loader_source`), grub needs a real `grub-install`-equivalent (target-specific generated
`grubx64.efi`, not a plain file copy) — out of scope for now; either shell out to `grub-install`
against the mounted ESP, or explicitly document grub as unsupported for genesis whole-disk mode.

**Genesis-produced disks don't actually boot into the installed system yet**: a plain partition
mount isn't how composefs systems boot — nothing in this project resolves `composefs.digest=<hash>`
(the kernel cmdline param `write_boot_entry` already writes) against the on-disk repository, mounts
the erofs image with fs-verity, and overlays `state/deploy/<digest>/etc/`. **Found a real, existing
upstream tool for exactly this**: `composefs-setup-root` (crates.io, same `composefs-rs`
project/version as our `composefs`/`composefs-boot` deps) — a Rust binary, not something we'd write
ourselves. What's still missing: the actual boot-time integration — the live VM's initramfs is
systemd-based (mkinitcpio's `systemd` hook, not classic busybox-style hooks), so this needs a
systemd unit ordered between `sysroot.mount` and `initrd-switch-root.target` (same role as ostree's
`ostree-prepare-root.service`), not a classic mkinitcpio hook script. Also unresolved: whether upac
needs to ship/package this integration itself, or whether it's expected to already exist on the
source distro (same assumption as the systemd-boot/rEFInd binary copy above) — needs checking
whether Arch/AUR already has a package for this.
