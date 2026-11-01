import { lazyRune } from "libs/ui/svelte/utils";
import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import { monitors } from "libs/ui/svelte/runes/monitors.svelte";

export const settings = lazyRune(() => invoke(SeelenCommand.StateGetSettings, { path: null }));
subscribe(SeelenEvent.StateSettingsChanged, settings.setByPayload);

export const focused = lazyRune(() => invoke(SeelenCommand.GetFocusedApp));
subscribe(SeelenEvent.GlobalFocusChanged, focused.setByPayload);

export const interactables = lazyRune(() => invoke(SeelenCommand.GetUserAppWindows));
subscribe(SeelenEvent.UserAppWindowsChanged, interactables.setByPayload);

export const virtualDesktops = lazyRune(() => invoke(SeelenCommand.StateGetVirtualDesktops));
subscribe(SeelenEvent.VirtualDesktopsChanged, virtualDesktops.setByPayload);

export const wallpapers = lazyRune(() => invoke(SeelenCommand.StateGetWallpapers));
subscribe(SeelenEvent.StateWallpapersChanged, wallpapers.setByPayload);

// fallible on the backend, a missing Windows wallpaper must not keep the widget from starting
export const windowsWallpaper = lazyRune(() => invoke(SeelenCommand.WallpaperGetWindows).catch(() => null));
subscribe(SeelenEvent.WallpaperWindowsChanged, windowsWallpaper.setByPayload);

export const performanceMode = lazyRune(() => invoke(SeelenCommand.StateGetPerformanceMode));
subscribe(SeelenEvent.StatePerformanceModeChanged, performanceMode.setByPayload);

export const players = lazyRune(() => invoke(SeelenCommand.GetMediaSessions));
subscribe(SeelenEvent.MediaSessions, players.setByPayload);

await Promise.all([
  settings.init(),
  focused.init(),
  interactables.init(),
  monitors.init(),
  virtualDesktops.init(),
  wallpapers.init(),
  windowsWallpaper.init(),
  performanceMode.init(),
  players.init(),
]);
