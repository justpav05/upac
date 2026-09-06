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

**UKI A/B boot (`upac-from.efi`/`upac-to.efi`) confirm-boot service not designed yet**: after a
successful boot, something needs to confirm once, swap `to`↔`from`, and set the normal persistent
boot order. Nothing calls `Booter::confirm_boot` anywhere yet; this belongs to a not-yet-designed
"confirm boot" systemd service, not genesis or the ordinary install/update pipeline.
