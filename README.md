<h1 align="center">
  <img src="documentation/images/logo.svg" width="44" align="top" alt="KestrelUI logo" />
  KestrelUI
</h1>

<p align="center">
  Customizable desktop environment for Windows — no accounts, no ads, no telemetry.
</p>

---

KestrelUI is a community fork of [Seelen UI](https://github.com/eythaann/seelen-ui). It keeps everything that made
Seelen great — the tiling window manager, the dock, the toolbar, widgets, themes, icon packs and wallpapers — and
removes everything that got in the way:

- **No account.** The login flow, plan tiers and the cloud vault are gone. Every feature works offline.
- **No ads.** No "Upgrade" banners, no news feed, no promo buttons in your Discord status.
- **No telemetry.** Nothing is sent home on startup. The only network requests are update checks and the showcase,
  both against public GitHub URLs.
- **Showcase.** Ready-made presets and community configs, installable in one click and reversible in one click.
  Publishing your own is a pull request to
  [kestrelui-showcase](https://github.com/Qwarwin4/KestrelUI-showcase).

## Status

Early fork. The first milestone is a clean, de-branded build at feature parity with Seelen UI 2.8.9. Then: bug
fixes, performance, and a KestrelUI-specific design and customization layer. See [ROADMAP.md](ROADMAP.md).

Compatibility: themes, widgets, plugins and icon packs built for Seelen UI keep working — the `@seelen/*` resource
ids and the `SeelenCommand` plugin API are intentionally left untouched.

## Building

Windows 10/11 only.

1. Run `setup.bat` once. It installs Visual Studio Build Tools (MSVC + Windows SDK), Rust, Node.js, Deno,
   Git and WebView2 through winget, then the pinned Rust toolchain.
2. Open a new terminal and run `build.bat`:

   | command             | result                                           |
   | ------------------- | ------------------------------------------------ |
   | `build.bat`         | debug installer in `target\debug\bundle\nsis`     |
   | `build.bat dev`     | run from sources with hot reload                 |
   | `build.bat release` | release installer in `target\release\bundle\nsis` |

Both scripts work from any folder, including an unpacked zip. `npm run doctor` checks the prerequisites and says
what is missing — the usual one is the Windows SDK (`rust-lld: could not open 'kernel32.lib'`).

Local release builds are unsigned and have auto-updates off.
Signed releases with auto-updates need your own key pair:

```sh
npx tauri signer generate -w kestrelui.key
```

Set `TAURI_SIGNING_PRIVATE_KEY` (+ password) and `KESTREL_PUBKEY` (the public key, base64) as repository
secrets; the release workflow embeds the public key into the binary for update and bundle-integrity checks.
`KESTREL_DISCORD_ID` (optional variable) enables Discord rich presence with your own Discord application.

## License

[GNU AGPL-3.0](LICENSE), same as upstream. KestrelUI contains code from Seelen UI, © Seelen Corp. and contributors.
See [NOTICE](NOTICE).
