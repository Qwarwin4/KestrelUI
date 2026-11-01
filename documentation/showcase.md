# Showcase

The showcase has no central server or registry. Every entry is a public GitHub repository with the topic
`kestrelui-config` and a `kestrelui.json` manifest in its root. The settings app finds them through GitHub search and
sorts them by stars.

## Publishing from the app

Showcase → Publish signs you in with GitHub (device flow, only the `public_repo` scope), takes a screenshot of your
desktop and creates `kestrelui-<name>` in your account with the manifest, a settings patch and the preview. Publishing
the same name again updates that repository.

Only dock, toolbar, window manager, theme and date settings are shared. Shortcuts, monitors, language and per-app rules
never leave your machine, and the app drops them from other people's presets on install as well.

## Publishing by hand

```
kestrelui.json
preview.jpg      16:9
settings.json    presets: partial settings merged into the user's
my-theme.yml     themes, widgets, plugins: `slu resource bundle theme ./my-theme`
```

```json
{
  "name": "My Setup",
  "kind": "preset",
  "description": "What it looks like and who it is for.",
  "tags": ["dock", "minimal"],
  "preview": "preview.jpg",
  "settings": "settings.json",
  "resources": ["my-theme.yml"],
  "resourceIds": ["@you/my-theme"]
}
```

`kind` is one of `preset`, `theme`, `widget`, `plugin`, `iconPack`, `wallpaper`. Add the `kestrelui-config` topic to the
repository and it shows up within a few minutes.

Widgets and plugins run their own code, so the app warns before installing them from anyone but the official account.

## Building

Sign-in needs a GitHub OAuth App with device flow enabled. Put its client id in `KESTREL_GITHUB_CLIENT_ID` at build time
(a repository variable for the release workflow). Without it browsing and installing still work, only publishing is off.
