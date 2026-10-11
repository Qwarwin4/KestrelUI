import { signal } from "@preact/signals";

export const REGISTRY = "https://raw.githubusercontent.com/Qwarwin4/winuirev-showcase/main/";
export const REGISTRY_REPO = "https://github.com/Qwarwin4/winuirev-showcase";

export type ShowcaseKind = "preset" | "theme" | "widget" | "plugin" | "iconPack" | "wallpaper";

export interface ShowcaseAsset {
  path: string;
  sha256: string;
}

export interface ShowcaseEntry {
  id: string;
  name: string;
  author: string;
  description: string;
  kind: ShowcaseKind;
  tags: string[];
  official: boolean;
  preview: string;
  updatedAt: string;
  resourceIds: string[];
  resources: ShowcaseAsset[];
  settings?: ShowcaseAsset;
  source?: string;
}

interface Index {
  version: number;
  entries: ShowcaseEntry[];
}

type State = { status: "idle" | "loading" | "error" | "ready"; entries: ShowcaseEntry[] };

export const showcase = signal<State>({ status: "idle", entries: [] });

let inflight: Promise<ShowcaseEntry[]> | null = null;

export function loadShowcase(force = false): Promise<ShowcaseEntry[]> {
  if (inflight && !force) return inflight;
  showcase.value = { ...showcase.value, status: "loading" };
  inflight = fetch(`${REGISTRY}index.json`, { cache: force ? "reload" : "default" })
    .then((r) => {
      if (!r.ok) throw new Error(`index: ${r.status}`);
      return r.json() as Promise<Index>;
    })
    .then((idx) => {
      if (idx.version !== 1) throw new Error("unsupported index version");
      showcase.value = { status: "ready", entries: idx.entries };
      return idx.entries;
    })
    .catch((e) => {
      console.error(e);
      showcase.value = { status: "error", entries: [] };
      inflight = null;
      return [];
    });
  return inflight;
}

export function previewUrl(e: ShowcaseEntry): string {
  return REGISTRY + e.preview;
}

export function entryForResource(entries: ShowcaseEntry[], resourceId: string): ShowcaseEntry | undefined {
  return entries.find((e) => e.resourceIds.includes(resourceId));
}
