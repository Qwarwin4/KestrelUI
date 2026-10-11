# WinUIRev roadmap

## 0 — Clean fork (done)

- Account system, Sky plan, cloud backup, telemetry, news feed, store integration and promo content removed
- Rebranded: name, bundle id `com.qwarwin.winuirev`, executables, pipes, mutexes, task scheduler entries,
  installer, icons — WinUIRev installs side by side with Seelen UI without fighting over the same names
- Release signing and Discord presence moved to build-time variables (`WINUIREV_PUBKEY`, `WINUIREV_DISCORD_ID`)
- Showcase: GitHub registry (`Qwarwin4/winuirev-showcase`), in-app page, one-click install, settings
  snapshots with undo, `winuirev.uri:showcase/<id>` deep links, update checks against the index

## 1 — First release

- [ ] Create `Qwarwin4/WinUIRev` and `Qwarwin4/winuirev-showcase` on GitHub, push both
- [ ] Generate the signing key pair, add repo secrets, tag `v2.8.9-rev.1`
- [ ] Windows smoke test: install, autostart, service, dock/toolbar/WM, settings, showcase install + undo
- [ ] Real screenshots for the two official presets
- [ ] Clean remaining Seelen-specific docs in `documentation/`

## 2 — Stability and speed

- [ ] Triage upstream open issues with the `bug` label, port fixes that apply
- [ ] Profile WebView2 memory per widget; merge low-traffic popups into shared webviews
- [ ] Startup time budget: measure integrity check + widget spawn, lazy-load rarely used widgets
- [ ] Crash reporting stays local: a "copy diagnostics" button instead of telemetry

## 3 — WinUIRev design and customization

- [ ] Own design system on top of the token layer (accent, surfaces, radius, motion presets)
- [ ] Live theme editor in settings: tweak tokens with instant preview, save as a theme, publish to showcase
- [ ] Per-workspace and per-monitor presets (switch the whole look with a shortcut)
- [ ] Preset diffs: show exactly which settings a showcase entry changes before applying
- [ ] Icon packs and wallpapers in the showcase as plain folders (no .slu)

## Upstream sync

`upstream` remote points at `eythaann/seelen-ui`. Merge upstream releases after reviewing them for new
account/telemetry/store code. Internal names (`@seelen/*` ids, `SeelenCommand`, `seelen-core`) are kept on
purpose to keep merges cheap and community resources compatible.
