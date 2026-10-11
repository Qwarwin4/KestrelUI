<h1 align="center">
  <img src="documentation/images/logo.svg" width="44" align="top" alt="WinUIRev logo" />
  WinUIRev
</h1>

<p align="center">
  Customizable desktop environment for Windows — no accounts, no ads, no telemetry.
</p>

---

WinUIRev is a community fork of [Seelen UI](https://github.com/eythaann/seelen-ui). It keeps everything that made
Seelen great — the tiling window manager, the dock, the toolbar, widgets, themes, icon packs and wallpapers — and
removes everything that got in the way:

- **No account.** The login flow, plan tiers and the cloud vault are gone. Every feature works offline.
- **No ads.** No "Upgrade" banners, no news feed, no promo buttons in your Discord status.
- **No telemetry.** Nothing is sent home on startup. The only network requests are update checks and the showcase,
  both against public GitHub URLs.
- **Showcase.** Ready-made presets and community configs, installable in one click and reversible in one click.
  Publishing your own is a pull request to
  [winuirev-showcase](https://github.com/Qwarwin4/winuirev-showcase).

## Status

Early fork. The first milestone is a clean, de-branded build at feature parity with Seelen UI 2.8.9. Then: bug
fixes, performance, and a WinUIRev-specific design and customization layer. See [ROADMAP.md](ROADMAP.md).

Compatibility: themes, widgets, plugins and icon packs built for Seelen UI keep working — the `@seelen/*` resource
ids and the `SeelenCommand` plugin API are intentionally left untouched.

## Building

Windows 10/11 only. Install once:

1. **Visual Studio Build Tools 2022** with the *Desktop development with C++* workload (MSVC + Windows SDK).
   Rust links against the SDK's `kernel32.lib` and friends, so without it every build script fails with
   `rust-lld: could not open 'kernel32.lib'`.
   ```powershell
   winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```
2. **Rust** via [rustup](https://rustup.rs). The pinned nightly from `rust-toolchain.toml` installs itself.
3. **Node.js 22+** and **Deno 2** (`winget install DenoLand.Deno`).
4. WebView2 Runtime (already present on Windows 11).

Open a **new** terminal after installing, then run each command on its own line:

```powershell
npm install
npm run dev
```

`npm install` starts with `npm run doctor`, which checks the prerequisites above and tells you what is missing.

Release builds need your own signing key pair:

```sh
npx tauri signer generate -w winuirev.key
```

Set `TAURI_SIGNING_PRIVATE_KEY` (+ password) and `WINUIREV_PUBKEY` (the public key, base64) as repository
secrets; the release workflow embeds the public key into the binary for update and bundle-integrity checks.
`WINUIREV_DISCORD_ID` (optional variable) enables Discord rich presence with your own Discord application.

## License

[GNU AGPL-3.0](LICENSE), same as upstream. WinUIRev contains code from Seelen UI, © Seelen Corp. and contributors.
See [NOTICE](NOTICE).
