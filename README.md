<p align="center">
  <img src="documentation/images/logo.svg" width="112" alt="KestrelUI logo" />
</p>

<h1 align="center">KestrelUI</h1>

<p align="center">
  Customizable desktop environment for Windows — no accounts, no ads, no telemetry.
</p>

<p align="center">
  <a href="https://discord.gg/ycmhfnmCqh"><img src="https://img.shields.io/badge/Discord-Join_server-5ef0a0?style=for-the-badge&logo=discord&logoColor=5ef0a0&labelColor=0b1410" height="40" alt="Discord: join the server"></a>
  <a href="https://t.me/qwarwin4git"><img src="https://img.shields.io/badge/Telegram-Follow-5ef0a0?style=for-the-badge&logo=telegram&logoColor=5ef0a0&labelColor=0b1410" height="40" alt="Telegram: follow the channel"></a>
  <a href="mailto:qwarwin@gmail.com"><img src="https://img.shields.io/badge/Gmail-Contact-5ef0a0?style=for-the-badge&logo=gmail&logoColor=5ef0a0&labelColor=0b1410" height="40" alt="Gmail: contact"></a>
</p>
<p align="center">
  <a href="https://github.com/Qwarwin4/KestrelUI"><img src="https://img.shields.io/badge/KestrelUI-v0.1.0-5ef0a0?style=for-the-badge&logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNTYgMjU2Ij4gPHBhdGggZD0iTTcwIDQ4aDMwdjE2MEg3MHoiIGZpbGw9IiM1ZWYwYTAiLz4gPHBhdGggZD0iTTEwNiAxMjQgMjA2IDQ2bC0xNiAyMiAxMi0yLTE4IDIyIDEwLTEtNTggNDd6IiBmaWxsPSIjNWVmMGEwIi8%2BIDxwYXRoIGQ9Ik0xMTAgMTMybDk0IDc2aC00MmwtNjItNTB6IiBmaWxsPSIjNWVmMGEwIiBvcGFjaXR5PSIuNjUiLz4gPC9zdmc%2B&logoColor=5ef0a0&labelColor=0b1410" height="40" alt="KestrelUI v0.1.0"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-AGPL--3.0-5ef0a0?style=for-the-badge&logo=gnu&logoColor=5ef0a0&labelColor=0b1410" height="40" alt="License: AGPL-3.0"></a>
</p>

<p align="center">
  <img src="documentation/images/screenshots/hero.webp" width="900" alt="KestrelUI settings window with the Neon Moss look">
</p>

---

KestrelUI is a community fork of [Seelen UI](https://github.com/eythaann/seelen-ui). It keeps everything that made
Seelen great — the tiling window manager, the dock, the toolbar, widgets, themes, icon packs and wallpapers — and
removes everything that got in the way:

- **No account.** The login flow, plan tiers and the cloud vault are gone. Every feature works offline.
- **No ads.** No "Upgrade" banners, no news feed, no review nags, no Discord status.
- **No telemetry.** Nothing is sent home on startup. The only network requests are update checks and the showcase, both
  against public GitHub URLs, plus the model provider you connect the assistant to, if any.
- **Showcase.** Community setups from GitHub, installable and reversible in one click. Publish yours from the app with a
  GitHub sign-in, see [documentation/showcase.md](documentation/showcase.md).
- **Assistant.** Connect GPT, Claude, OpenRouter or any OpenAI-compatible API with your own key and ask it to restyle
  things or explain them. Changes are applied through a limited set of settings and can be undone in one click; API keys
  stay in the Windows credential vault.

## Screenshots

<table>
  <tr>
    <td width="50%"><img src="documentation/images/screenshots/theme-settings.webp" alt="Neon Moss theme settings"><p align="center">Neon Moss: neon color, glass, glow, neon on the dock and toolbar</p></td>
    <td width="50%"><img src="documentation/images/screenshots/start-button.webp" alt="Start button presets"><p align="center">Start button: pixel presets or your own picture</p></td>
  </tr>
  <tr>
    <td width="50%"><img src="documentation/images/screenshots/icon-packs.webp" alt="Icon packs with local import"><p align="center">Icon packs from a folder of pictures, resources from disk</p></td>
    <td width="50%"><img src="documentation/images/screenshots/assistant.webp" alt="Assistant"><p align="center">Assistant over your own GPT, Claude or OpenRouter key</p></td>
  </tr>
  <tr>
    <td width="50%"><img src="documentation/images/screenshots/monitors.webp" alt="Monitors and wallpapers"><p align="center">Monitors and wallpapers, the Windows wallpaper by default</p></td>
    <td width="50%"><img src="documentation/images/screenshots/neon-color.webp" alt="General settings"><p align="center">General: startup, language, neon presets and color picker</p></td>
  </tr>
</table>

## Status

Early fork. The first milestone is a clean, de-branded build at feature parity with Seelen UI 2.8.11. Then: bug fixes,
performance, and a KestrelUI-specific design and customization layer. See [ROADMAP.md](ROADMAP.md).

Compatibility: themes, widgets, plugins and icon packs built for Seelen UI keep working — the `@seelen/*` resource ids
and the `SeelenCommand` plugin API are intentionally left untouched.

## Building

Windows 10/11 only. Run `build.bat` (double-click works) and press a number in the menu, or pass the mode as an
argument. It installs whatever is missing through winget — Visual Studio Build Tools with the Windows SDK, Rust,
Node.js, Deno, Git, WebView2 — and then builds:

| menu / command        | result                                            |
| --------------------- | ------------------------------------------------- |
| 1 `build.bat debug`   | debug installer in `target\debug\bundle\nsis`     |
| 2 `build.bat release` | release installer in `target\release\bundle\nsis` |
| 3 `build.bat dev`     | run from sources with hot reload                  |
| 4 `build.bat deps`    | only install and check the build tools            |

The window stays open at the end and the full output goes to `build.log`; on failure the last lines are shown right
away.

Local release builds are unsigned and have auto-updates off. Signed releases with auto-updates need your own key pair:

```sh
npx tauri signer generate -w kestrelui.key
```

Set `TAURI_SIGNING_PRIVATE_KEY` (+ password) and `KESTREL_PUBKEY` (the public key, base64) as repository secrets; the
release workflow embeds the public key into the binary for update and bundle-integrity checks.

## License

[GNU AGPL-3.0](LICENSE), same as upstream. KestrelUI contains code from Seelen UI, © Seelen Corp. and contributors. See
[NOTICE](NOTICE).
