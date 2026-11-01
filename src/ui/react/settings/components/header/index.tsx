import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { ResourceText } from "libs/ui/react/components/ResourceText/index.tsx";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { Button } from "antd";
import type { ComponentChildren } from "preact";
import React from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation, useSearchParams } from "react-router";

import { SHORT_NAMES } from "../navigation/index.tsx";
import { RouteExtraInfo } from "./ExtraInfo.tsx";
import { UpdateButton } from "./UpdateButton.tsx";
import cs from "./index.module.css";
import { themes as themeList, widgets as widgetList } from "../../state/resources.ts";
import { hasChanges, needRestart, restoreToLastSaved, saveSettings } from "../../state/mod.ts";

interface PageInfo {
  /** route segments, for the `~/a/b` path in the top bar */
  parts: string[];
  title: ComponentChildren;
  /** i18n key of the line under the title, if the page has one */
  subtitle?: string;
}

function usePage(): PageInfo {
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const { t } = useTranslation();
  const parts = location.pathname === "/" ? ["showcase"] : location.pathname.split("/").filter(Boolean);

  if (parts[0] === "widget" || parts[0] === "theme") {
    const id = searchParams.get("id") ?? "";
    const list = parts[0] === "widget" ? widgetList.value : themeList.value;
    const found = list.find((r) => r.id === id);
    const short = parts[0] === "widget" ? SHORT_NAMES[id] : undefined;
    return {
      parts: [parts[0], id.split("/").pop() ?? id],
      title: short ? t(`nav.widgets.${short}`) : found ? <ResourceText text={found.metadata.displayName} /> : id,
    };
  }

  if (parts[0] === "wallpaper") {
    return { parts: ["resources", "wallpaper"], title: t("header.labels.wallpaper") };
  }

  const last = parts[parts.length - 1]!;
  return {
    parts,
    title: t(`header.labels.${last}`),
    subtitle: parts.length === 1 ? `pages.${last}` : undefined,
  };
}

export const Header = () => {
  const { t } = useTranslation();
  const location = useLocation();
  const page = usePage();

  const SaveOrQuit = async () => {
    if (hasChanges.value) {
      await saveSettings();
      if (needRestart.value) {
        await invoke(SeelenCommand.AppRestart);
      }
    } else {
      await getCurrentWebviewWindow().close();
    }
  };

  const ExtraInfo = RouteExtraInfo[location.pathname];

  return (
    <div className={cs.header}>
      <div className={cs.path}>
        <span className={cs.home}>~</span>
        {page.parts.map((part, idx) => (
          <React.Fragment key={part + idx}>
            <span className={cs.slash}>/</span>
            {idx < page.parts.length - 1
              ? (
                <NavLink to={`/${page.parts.slice(0, idx + 1).join("/")}`} className={cs.part}>
                  {part}
                </NavLink>
              )
              : <span className={cs.part}>{part}</span>}
          </React.Fragment>
        ))}
        {ExtraInfo && <ExtraInfo />}
      </div>

      <div className={cs.actions}>
        <UpdateButton />
        {hasChanges.value && (
          <Button onClick={restoreToLastSaved}>
            {t("cancel")}
          </Button>
        )}
        <Button type={hasChanges.value ? "primary" : "default"} onClick={SaveOrQuit}>
          {hasChanges.value ? (needRestart.value ? t("save_and_restart") : t("save")) : t("close")}
        </Button>
      </div>
    </div>
  );
};

/** Big title at the top of each page, like the mockup. Pages with their own hero skip it. */
export function PageHeading() {
  const { t } = useTranslation();
  const location = useLocation();
  const page = usePage();

  if (location.pathname === "/" || location.pathname === "/assistant") {
    return null;
  }

  const subtitle = page.subtitle ? t(page.subtitle, { defaultValue: "" }) : "";

  return (
    <div className={cs.heading}>
      <h1>{page.title}</h1>
      {subtitle && <p>{subtitle}</p>}
    </div>
  );
}
