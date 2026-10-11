import { lazySignal } from "libs/ui/react/utils/LazySignal";
import { invoke, SeelenCommand, SeelenEvent, subscribe } from "@seelen-ui/lib";
import type { ResourceKind, ResourceMetadata, ResourceText } from "@seelen-ui/lib/types";
import { ResourceKind as RK } from "@seelen-ui/lib/types";
import { computed, effect, signal } from "@preact/signals";
import { getResourceText } from "@shared";
import { language } from "./mod";
import { entryForResource, loadShowcase } from "../modules/showcase/registry.ts";

function SorterByDisplayName(
  a: { metadata: { displayName: ResourceText } },
  b: { metadata: { displayName: ResourceText } },
): number {
  const aDisplayName = getResourceText(a.metadata.displayName, language.value);
  const bDisplayName = getResourceText(b.metadata.displayName, language.value);

  return aDisplayName.localeCompare(bDisplayName, language.value);
}

const _widgets = lazySignal(() => invoke(SeelenCommand.StateGetWidgets));
subscribe(SeelenEvent.StateWidgetsChanged, _widgets.setByPayload);
await _widgets.init();

const _plugins = lazySignal(() => invoke(SeelenCommand.StateGetPlugins));
subscribe(SeelenEvent.StatePluginsChanged, _plugins.setByPayload);
await _plugins.init();

const _themes = lazySignal(() => invoke(SeelenCommand.StateGetThemes));
subscribe(SeelenEvent.StateThemesChanged, _themes.setByPayload);
await _themes.init();

const _iconPacks = lazySignal(() => invoke(SeelenCommand.StateGetIconPacks));
subscribe(SeelenEvent.StateIconPacksChanged, _iconPacks.setByPayload);
await _iconPacks.init();

const _wallpapers = lazySignal(() => invoke(SeelenCommand.StateGetWallpapers));
subscribe(SeelenEvent.StateWallpapersChanged, _wallpapers.setByPayload);
await _wallpapers.init();

export const widgets = computed(() => _widgets.value.toSorted(SorterByDisplayName));
export const plugins = computed(() => _plugins.value.toSorted(SorterByDisplayName));
export const themes = computed(() => _themes.value.toSorted(SorterByDisplayName));
export const iconPacks = computed(() => _iconPacks.value.toSorted(SorterByDisplayName));
export const wallpapers = computed(() => _wallpapers.value.sort(SorterByDisplayName));

// check for updates
export type ResourceWithKind = { id: string; metadata: ResourceMetadata; kind: ResourceKind };
export type ResourceUpdate = ResourceWithKind & { showcaseId: string };
export const resourcesWithUpdate = signal<ResourceUpdate[]>([]);

effect(() => {
  const local: ResourceWithKind[] = [
    ...widgets.value.map((r) => ({ id: r.id, metadata: r.metadata, kind: RK.Widget })),
    ...themes.value.map((r) => ({ id: r.id, metadata: r.metadata, kind: RK.Theme })),
    ...plugins.value.map((r) => ({ id: r.id, metadata: r.metadata, kind: RK.Plugin })),
    ...iconPacks.value.map((r) => ({ id: r.id, metadata: r.metadata, kind: RK.IconPack })),
    ...wallpapers.value.map((r) => ({ id: r.id, metadata: r.metadata, kind: RK.Wallpaper })),
  ];

  if (local.length === 0) {
    resourcesWithUpdate.value = [];
    return;
  }

  loadShowcase().then((entries) => {
    resourcesWithUpdate.value = local.flatMap((r) => {
      const entry = entryForResource(entries, r.id);
      if (!entry || new Date(entry.updatedAt) <= new Date(r.metadata.writtenAt)) return [];
      return [{ ...r, showcaseId: entry.id }];
    });
  });
});
