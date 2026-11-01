# KestrelUI roadmap

## 0 — Clean fork (done)

- Account system, Sky plan, cloud backup, telemetry, news feed, store integration and promo content removed
- Rebranded: name, bundle id `com.qwarwin.kestrelui`, executables, pipes, mutexes, task scheduler entries,
  installer, icons — KestrelUI installs side by side with Seelen UI without fighting over the same names
- Release signing and Discord presence moved to build-time variables (`KESTREL_PUBKEY`, `KESTREL_DISCORD_ID`)
- Showcase: community repos tagged `kestrelui-config` found through GitHub search, publishing via GitHub sign-in,
  one-click install, settings snapshots with undo, `kestrelui.uri:showcase/<owner>/<repo>` deep links
- Neon Moss default theme: dark glass, neon and icon colors from presets, HEX, RGB or a color square

## 1 — First release

- [ ] Rename the GitHub repo to `Qwarwin4/KestrelUI`, create the GitHub OAuth App for publishing
- [ ] Generate the signing key pair, add repo secrets, tag `v0.1.0-a`
- [ ] Windows smoke test: install, autostart, service, dock/toolbar/WM, settings, showcase install + undo
- [ ] Publish the official presets from the app
- [ ] Clean remaining Seelen-specific docs in `documentation/`

## 2 — Stability and speed

- [ ] Triage upstream open issues with the `bug` label, port fixes that apply
- [ ] Profile WebView2 memory per widget; merge low-traffic popups into shared webviews
- [ ] Startup time budget: measure integrity check + widget spawn, lazy-load rarely used widgets
- [ ] Crash reporting stays local: a "copy diagnostics" button instead of telemetry

## 3 — KestrelUI design and customization

- [ ] Own design system on top of the token layer (accent, surfaces, radius, motion presets)
- [ ] Live theme editor in settings: tweak tokens with instant preview, save as a theme, publish to showcase
- [ ] Per-workspace and per-monitor presets (switch the whole look with a shortcut)
- [ ] Preset diffs: show exactly which settings a showcase entry changes before applying
- [ ] Icon packs and wallpapers in the showcase as plain folders (no .slu)

## Upstream sync

`upstream` remote points at `eythaann/seelen-ui`. Merge upstream releases after reviewing them for new
account/telemetry/store code. Internal names (`@seelen/*` ids, `SeelenCommand`, `seelen-core`) are kept on
purpose to keep merges cheap and community resources compatible.
