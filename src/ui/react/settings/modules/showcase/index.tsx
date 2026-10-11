import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import type { IconName } from "libs/ui/icons.ts";
import { cx } from "libs/ui/react/utils/styling.ts";
import { Button, Empty, Input, Segmented, Skeleton, Tooltip } from "antd";
import { useEffect, useMemo, useState } from "preact/hooks";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";

import { installFromShowcase, installing, undoLastPreset } from "./actions.ts";
import { loadShowcase, previewUrl, REGISTRY_REPO, showcase, type ShowcaseEntry, type ShowcaseKind } from "./registry.ts";
import { iconPacks, plugins, themes, wallpapers, widgets } from "../../state/resources.ts";
import cs from "./index.module.css";

const kinds: (ShowcaseKind | "all")[] = ["all", "preset", "theme", "widget", "plugin", "iconPack", "wallpaper"];

const kindIcon: Record<ShowcaseKind, IconName> = {
  preset: "LuLayers",
  theme: "IoColorPalette",
  widget: "BiSolidWidget",
  plugin: "BsPlugin",
  iconPack: "LuPackage",
  wallpaper: "LuWallpaper",
};

export function Showcase() {
  const { t } = useTranslation();
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const kind = (params.get("kind") as ShowcaseKind | null) ?? "all";

  useEffect(() => {
    loadShowcase();
  }, []);

  const installed = useMemo(() => {
    const ids = new Set<string>();
    for (const list of [widgets.value, themes.value, plugins.value, iconPacks.value, wallpapers.value]) {
      for (const r of list) ids.add(r.id);
    }
    return ids;
  }, [widgets.value, themes.value, plugins.value, iconPacks.value, wallpapers.value]);

  const { status, entries } = showcase.value;
  const q = query.trim().toLowerCase();
  const shown = entries
    .filter((e) => kind === "all" || e.kind === kind)
    .filter((e) =>
      !q || e.name.toLowerCase().includes(q) || e.author.toLowerCase().includes(q) ||
      e.tags.some((tag) => tag.toLowerCase().includes(q))
    )
    .toSorted((a, b) => Number(b.official) - Number(a.official) || b.updatedAt.localeCompare(a.updatedAt));

  return (
    <div className={cs.page}>
      <header className={cs.hero}>
        <div>
          <h1>{t("showcase.title")}</h1>
          <p>{t("showcase.subtitle")}</p>
        </div>
        <div className={cs.heroActions}>
          <Tooltip title={t("showcase.undo_tooltip")}>
            <Button onClick={undoLastPreset}>
              <Icon iconName="LuUndo2" />
              {t("showcase.undo")}
            </Button>
          </Tooltip>
          <Button type="primary" href={`${REGISTRY_REPO}/blob/main/CONTRIBUTING.md`} target="_blank">
            <Icon iconName="LuUpload" />
            {t("showcase.publish")}
          </Button>
        </div>
      </header>

      <div className={cs.toolbar}>
        <Input
          type="search"
          allowClear
          placeholder={t("showcase.search")}
          value={query}
          onChange={(e) => setQuery((e.target as HTMLInputElement).value)}
        />
        <Segmented
          value={kind}
          onChange={(v) => setParams(v === "all" ? {} : { kind: String(v) })}
          options={kinds.map((k) => ({ value: k, label: t(`showcase.kind.${k}`) }))}
        />
        <Tooltip title={t("showcase.refresh")}>
          <Button type="text" onClick={() => loadShowcase(true)} loading={status === "loading"}>
            <Icon iconName="IoReload" />
          </Button>
        </Tooltip>
      </div>

      {status === "error" && (
        <Empty description={t("showcase.offline")}>
          <Button onClick={() => loadShowcase(true)}>{t("showcase.retry")}</Button>
        </Empty>
      )}

      {status === "ready" && shown.length === 0 && <Empty description={t("showcase.empty")} />}

      <div className={cs.grid}>
        {(status === "loading" || status === "idle") &&
          Array.from({ length: 8 }).map((_, i) => (
            <div key={i} className={cs.card}>
              <Skeleton.Image active className={cs.skeletonImage} />
              <Skeleton active paragraph={{ rows: 1 }} />
            </div>
          ))}

        {shown.map((e) => (
          <Card key={e.id} entry={e} installed={e.resourceIds.length > 0 && e.resourceIds.every((id) => installed.has(id))} />
        ))}
      </div>
    </div>
  );
}

function Card({ entry, installed }: { entry: ShowcaseEntry; installed: boolean }) {
  const { t } = useTranslation();
  const busy = installing.value === entry.id;

  return (
    <article className={cs.card}>
      <figure className={cs.preview}>
        <img src={previewUrl(entry)} loading="lazy" alt={entry.name} />
        <span className={cs.kind}>
          <Icon iconName={kindIcon[entry.kind]} />
          {t(`showcase.kind.${entry.kind}`)}
        </span>
        {entry.official && (
          <span className={cs.official}>
            <Icon iconName="LuBadgeCheck" />
          </span>
        )}
      </figure>

      <div className={cs.body}>
        <h3>{entry.name}</h3>
        <a className={cs.author} href={`https://github.com/${entry.author}`} target="_blank">
          @{entry.author}
        </a>
        <p className={cs.description}>{entry.description}</p>
        {entry.tags.length > 0 && (
          <div className={cs.tags}>
            {entry.tags.slice(0, 4).map((tag) => <span key={tag}>#{tag}</span>)}
          </div>
        )}
      </div>

      <footer className={cs.footer}>
        {entry.source && (
          <Button type="text" href={entry.source} target="_blank">
            <Icon iconName="LuGithub" />
          </Button>
        )}
        <Button
          type={installed ? "default" : "primary"}
          className={cx(cs.install, { [cs.done!]: installed })}
          loading={busy}
          disabled={!!installing.value && !busy}
          onClick={() => installFromShowcase(entry.id)}
        >
          {installed ? t("showcase.reinstall") : entry.kind === "preset" ? t("showcase.apply") : t("showcase.install")}
        </Button>
      </footer>
    </article>
  );
}
