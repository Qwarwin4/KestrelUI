import { signal } from "@preact/signals";

// every showcase entry is someone's public repo tagged with this topic and a
// kestrelui.json manifest in its root; nothing is hosted by the project itself
export const TOPIC = "kestrelui-config";
const OFFICIAL = new Set(["qwarwin4"]);
const CACHE_KEY = "showcase-cache-v2";
const CACHE_TTL = 10 * 60 * 1000;

export type ShowcaseKind = "preset" | "theme" | "widget" | "plugin" | "iconPack" | "wallpaper";

export interface ShowcaseEntry {
  id: string;
  name: string;
  author: string;
  description: string;
  kind: ShowcaseKind;
  tags: string[];
  official: boolean;
  stars: number;
  preview: string;
  updatedAt: string;
  resourceIds: string[];
  source: string;
}

interface Manifest {
  name?: string;
  kind?: ShowcaseKind;
  description?: string;
  tags?: string[];
  preview?: string;
  resourceIds?: string[];
}

interface SearchRepo {
  full_name: string;
  html_url: string;
  description: string | null;
  stargazers_count: number;
  pushed_at: string;
  owner: { login: string };
}

type State = { status: "idle" | "loading" | "error" | "ready"; entries: ShowcaseEntry[] };

export const showcase = signal<State>({ status: "idle", entries: [] });

let inflight: Promise<ShowcaseEntry[]> | null = null;

export function raw(repo: string, path: string) {
  return `https://raw.githubusercontent.com/${repo}/HEAD/${path}`;
}

function readCache(): ShowcaseEntry[] | null {
  try {
    const c = JSON.parse(localStorage.getItem(CACHE_KEY) ?? "null");
    if (c && Date.now() - c.at < CACHE_TTL) return c.entries;
  } catch {}
  return null;
}

function writeCache(entries: ShowcaseEntry[]) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), entries }));
  } catch {}
}

async function toEntry(repo: SearchRepo): Promise<ShowcaseEntry | null> {
  const res = await fetch(raw(repo.full_name, "kestrelui.json"));
  if (!res.ok) return null;
  const m: Manifest = await res.json();
  if (!m.kind) return null;

  return {
    id: repo.full_name,
    name: m.name || repo.full_name.split("/")[1]!,
    author: repo.owner.login,
    description: m.description || repo.description || "",
    kind: m.kind,
    tags: (m.tags ?? []).slice(0, 8),
    official: OFFICIAL.has(repo.owner.login.toLowerCase()),
    stars: repo.stargazers_count,
    preview: m.preview ? raw(repo.full_name, m.preview) : "",
    updatedAt: repo.pushed_at,
    resourceIds: m.resourceIds ?? [],
    source: repo.html_url,
  };
}

async function search(): Promise<ShowcaseEntry[]> {
  // unauthenticated search allows 10 requests a minute, hence the cache
  const url = `https://api.github.com/search/repositories?q=topic:${TOPIC}&sort=stars&order=desc&per_page=100`;
  const res = await fetch(url, { headers: { Accept: "application/vnd.github+json" } });
  if (!res.ok) throw new Error(`github search: ${res.status}`);
  const { items } = (await res.json()) as { items: SearchRepo[] };
  const entries = await Promise.allSettled(items.map(toEntry));
  return entries.flatMap((r) => (r.status === "fulfilled" && r.value ? [r.value] : []));
}

export function loadShowcase(force = false): Promise<ShowcaseEntry[]> {
  if (inflight && !force) return inflight;

  const cached = force ? null : readCache();
  if (cached) {
    showcase.value = { status: "ready", entries: cached };
    return (inflight = Promise.resolve(cached));
  }

  showcase.value = { ...showcase.value, status: "loading" };
  inflight = search()
    .then((entries) => {
      writeCache(entries);
      showcase.value = { status: "ready", entries };
      return entries;
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
  return e.preview;
}

export function entryForResource(entries: ShowcaseEntry[], resourceId: string): ShowcaseEntry | undefined {
  return entries.find((e) => e.resourceIds.includes(resourceId));
}

// code from strangers runs on the desktop only for these kinds
export function runsCode(e: ShowcaseEntry): boolean {
  return e.kind === "widget" || e.kind === "plugin";
}
