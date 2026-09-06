<h1 align="center">✅ TODO</h1> 

Near-term, concrete items. See `ROADMAP.md` for the bigger picture.

## upac-cli

- `user/upac-cli/data/` (`.desktop`, `upac-mime.xml`, `.policy`) reference `Icon=upac`/`icon_name=upac`,
  but there's no actual icon asset (SVG/PNG) yet, and no install step wiring it into
  `/usr/share/icons/hicolor/...`. Needs real artwork before packaging.

## upac-lib

Test-coverage pass in progress. The entire non-command core is covered (`errors.rs`/`lock.rs`/
`search.rs`/`fs.rs`/`orchestrator/*`/`database/*`/`deploy/*`/`scripts/*`/`composefs/*`/`config/*`/
`boot/*`/`plugin/decoder/{error,manifest,triggers}.rs`/`plugin/boot/{error,manifest}.rs`), except
`plugin/decoder/unpack.rs`/`plugin/decoder/mod.rs`/`plugin/boot/mod.rs` (need a real dlopen'd/
`builtin-*` plugin) and `deploy/esp.rs` (real mount table) — both explicit, justified skips. Every
`mutated`/`unmutated` command's own `<Command>Error` enum is also now covered (inline tests next to
each `error.rs`, since `mutated`/`unmutated` aren't `pub`) — only each variant's own logic, not the
macro-generated `Common(...)` delegation shared with `errors.rs`'s already-tested `CommonError`.
Remaining: the `Stage::run()` bodies themselves — each needs a real composefs `Repository`/`Deploy`/
database in context, likely out of scope for unit tests unless a pure-logic helper turns out to be
extractable.

**Two standalone boot-time services still need to be built** — neither is upac-lib/upac-cli code,
both run on the installed system itself, outside anything `up`/`up-sp` invokes:

- **composefs-mount boot hook**: nothing yet resolves `composefs.digest=<hash>` (the kernel cmdline
  param `write_boot_entry` already writes) against the on-disk repository, mounts the erofs image
  with fs-verity, and overlays `state/deploy/<digest>/etc/` — without this, a genesis-produced disk's
  firmware boots the kernel, but the initramfs has no way to actually assemble the root. The upstream
  tool for this already exists (`composefs-setup-root`, crates.io, same `composefs-rs` project as
  our `composefs`/`composefs-boot` deps) — what's missing is the systemd-unit integration (ordered
  between `sysroot.mount` and `initrd-switch-root.target`, same role as ostree's
  `ostree-prepare-root.service`; the live VM's initramfs is systemd-based, not classic mkinitcpio
  hooks). Also unresolved: whether upac ships/packages this integration itself or expects it to
  already exist on the source distro.
- **UKI A/B confirm-boot service**: after a successful boot, something needs to confirm once, swap
  `to`↔`from`, and set the normal persistent boot order. Nothing calls `Booter::confirm_boot`
  anywhere yet.

**`lib/lib/src/plugin/` needs a readability pass**: grown too dense to follow, `plugin/boot/mod.rs`
in particular (360 lines — static-link wiring for all 4 plugins, dynamic dlopen loading, and the
`BootPlugin` public API all crammed into one file). Split by concern (mirror the existing
folder-per-logical-unit convention used elsewhere in this crate) and simplify — right now it takes
real effort to follow what calls what.
