import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { ResourceText } from "libs/ui/react/components/ResourceText/index.tsx";
import { getResourceText } from "libs/ui/react/utils/index.ts";
import { Tooltip } from "antd";
import type { ComponentChildren } from "preact";
import { memo, useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation } from "react-router";

import { cx } from "../../modules/shared/utils/app.ts";
import { displayVersion, EnvConfig } from "../../modules/shared/config/infra.ts";
import { githubUser } from "../../modules/showcase/publish.tsx";

import { RouteIcons, RoutePath } from "./routes.tsx";
import cs from "./index.module.css";
import { resourcesWithUpdate, themes, widgets } from "../../state/resources.ts";
import { settings } from "../../state/mod.ts";

// the built-in widgets get short names in the menu, the rest keep their resource names
export const SHORT_NAMES: Record<string, string> = {
  "@seelen/weg": "dock",
  "@seelen/fancy-toolbar": "toolbar",
  "@seelen/window-manager": "tiling",
  "@seelen/wallpaper-manager": "wallpaper",
  "@seelen/apps-menu": "start",
  "@seelen/task-switcher": "switcher",
};

export const Navigation = memo(() => {
  const [collapsed, setCollapsed] = useState(false);
  const { t, i18n } = useTranslation();
  const location = useLocation();
  const search = new URLSearchParams(location.search);

  const hasResourceUpdates = resourcesWithUpdate.value.length > 0;
  const activeThemes = settings.value.activeThemes;
  const themesDirectAccess = themes.value
    .filter((theme) => theme.settings.length && activeThemes.includes(theme.id))
    .toSorted((a, b) =>
      getResourceText(a.metadata.displayName, i18n.language).localeCompare(
        getResourceText(b.metadata.displayName, i18n.language),
        i18n.language,
      )
    );

  const route = (path: RoutePath) => (
    <Item
      key={path}
      to={path}
      active={path === RoutePath.Home ? location.pathname === "/" : location.pathname.startsWith(path)}
      collapsed={collapsed}
      label={t(path === RoutePath.Home ? "header.labels.showcase" : `header.labels.${path.replace("/", "")}`)}
      icon={RouteIcons[path]}
      badge={path === RoutePath.Resource && hasResourceUpdates}
    />
  );

  return (
    <nav className={cx(cs.navigation, { [cs.collapsed!]: collapsed })} aria-label={t("nav.label")}>
      <div className={cs.brand}>
        <span className={cs.logo} aria-hidden="true">
          <span style={{ maskImage: "url(./logo-mark.svg)" }} />
        </span>
        <div className={cs.brandText}>
          <b>KestrelUI</b>
          <span>v{displayVersion(EnvConfig.version)}</span>
        </div>
        <button
          type="button"
          className={cs.collapse}
          onClick={() => setCollapsed(!collapsed)}
          aria-label={t(collapsed ? "nav.expand" : "nav.collapse")}
        >
          <Icon iconName={collapsed ? "LuPanelLeftOpen" : "LuPanelLeftClose"} />
        </button>
      </div>

      <div className={cs.body}>
        <Group label={t("nav.groups.main")} collapsed={collapsed}>
          {[RoutePath.Home, RoutePath.Assistant, RoutePath.General, RoutePath.Resource, RoutePath.Security].map(
            route,
          )}
        </Group>

        <Group label={t("nav.groups.desktop")} collapsed={collapsed}>
          {widgets.value
            .filter((widget) => !widget.hidden)
            .map((widget) => {
              const short = SHORT_NAMES[widget.id];
              return (
                <Item
                  key={widget.id}
                  to={`/widget?${new URLSearchParams({ id: widget.id })}`}
                  active={location.pathname === "/widget" && search.get("id") === widget.id}
                  collapsed={collapsed}
                  label={short ? t(`nav.widgets.${short}`) : <ResourceText text={widget.metadata.displayName} />}
                  icon={<Icon iconName={(widget.icon as any) || "BiSolidWidget"} />}
                />
              );
            })}
        </Group>

        {!!themesDirectAccess.length && (
          <Group label={t("nav.groups.themes")} collapsed={collapsed}>
            {themesDirectAccess.map((theme) => (
              <Item
                key={theme.id}
                to={`/theme?${new URLSearchParams({ id: theme.id })}`}
                active={location.pathname === "/theme" && search.get("id") === theme.id}
                collapsed={collapsed}
                label={<ResourceText text={theme.metadata.displayName} />}
                icon={<Icon iconName="BiSolidPalette" />}
              />
            ))}
          </Group>
        )}

        <Group label={t("nav.groups.system")} collapsed={collapsed}>
          {[RoutePath.SettingsByMonitor, RoutePath.SettingsByApplication, RoutePath.Shortcuts, RoutePath.DevTools]
            .map(route)}
        </Group>
      </div>

      <Account collapsed={collapsed} active={location.pathname === RoutePath.Extras} />
    </nav>
  );
});

function Group({ label, collapsed, children }: { label: string; collapsed: boolean; children: ComponentChildren }) {
  return (
    <div className={cs.group}>
      {collapsed ? <span className={cs.groupRule} /> : <span className={cs.groupLabel}>{label}</span>}
      {children}
    </div>
  );
}

interface ItemProps {
  to: string;
  active: boolean;
  collapsed: boolean;
  icon?: ComponentChildren;
  label: ComponentChildren;
  badge?: boolean;
}

function Item({ to, icon, label, active, collapsed, badge }: ItemProps) {
  return (
    <Tooltip placement="right" title={collapsed ? label : null}>
      <NavLink to={to} className={cx(cs.item, { [cs.active!]: active })}>
        <span className={cs.icon}>
          {icon}
          {badge && <span className={cs.badge} />}
        </span>
        <span className={cs.label}>{label}</span>
      </NavLink>
    </Tooltip>
  );
}

/** GitHub account when signed in for publishing, otherwise a link to "about" */
function Account({ collapsed, active }: { collapsed: boolean; active: boolean }) {
  const { t } = useTranslation();
  const user = githubUser.value;

  return (
    <Tooltip placement="right" title={collapsed ? t("header.labels.extras") : null}>
      <NavLink to={RoutePath.Extras} className={cx(cs.account, { [cs.active!]: active })}>
        {user
          ? <img className={cs.avatar} src={user.avatarUrl} alt="" />
          : (
            <span className={cs.avatar}>
              <Icon iconName="LuInfo" />
            </span>
          )}
        <span className={cs.accountText}>
          <b>{user ? `@${user.login}` : "KestrelUI"}</b>
          <span>{user ? t("nav.github") : t("header.labels.extras")}</span>
        </span>
      </NavLink>
    </Tooltip>
  );
}
