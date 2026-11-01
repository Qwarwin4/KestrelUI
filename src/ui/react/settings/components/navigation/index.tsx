import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { ResourceText } from "libs/ui/react/components/ResourceText/index.tsx";
import { getResourceText } from "libs/ui/react/utils/index.ts";
import { Tooltip } from "antd";
import { memo, useState } from "react";
import { useTranslation } from "react-i18next";
import { NavLink, useLocation } from "react-router";

import { cx } from "../../modules/shared/utils/app.ts";

import { RouteIcons, RoutePath } from "./routes.tsx";
import cs from "./index.module.css";
import { resourcesWithUpdate, themes, widgets } from "../../state/resources.ts";
import { settings } from "../../state/mod.ts";

export const Navigation = memo(() => {
  const [collapsed, setCollapsed] = useState(false);

  const activeThemes = settings.value.activeThemes;

  const { t, i18n } = useTranslation();
  const location = useLocation();

  const hasResourceUpdates = resourcesWithUpdate.value.length > 0;

  const Mapper = (route: RoutePath | null) => {
    if (!route) return null;
    return (
      <Item
        key={route}
        route={route}
        isActive={location.pathname.startsWith(route)}
        collapsed={collapsed}
        label={t(`header.labels.${route.replace("/", "")}`)}
        icon={RouteIcons[route]}
        badge={route === RoutePath.Resource ? hasResourceUpdates : false}
      />
    );
  };

  const themesDirectAccess = themes.value.filter(
    (theme) => theme.settings.length && activeThemes.includes(theme.id),
  );

  const advanceGroup = [
    RoutePath.SettingsByMonitor,
    RoutePath.SettingsByApplication,
    RoutePath.Shortcuts,
  ];
  const devGroup = [RoutePath.DevTools];

  return (
    <div
      className={cx(cs.navigation, {
        [cs.collapsed!]: collapsed,
      })}
    >
      <div className={cs.header} onClick={() => setCollapsed(!collapsed)}>
        <span className={cs.logo} aria-hidden="true">
          <span style={{ maskImage: "url(./logo-mark.svg)" }} />
        </span>
        <h1>KestrelUI</h1>
        <Icon className={cs.chevron} iconName="FaChevronLeft" />
      </div>

      <div className={cs.body}>
        <div className={cs.group}>
          <Item
            route={RoutePath.Home}
            isActive={location.pathname === "/"}
            label={t("header.labels.showcase")}
            icon={<Icon iconName="LuStore" />}
            collapsed={collapsed}
          />
          {[RoutePath.Assistant, RoutePath.General, RoutePath.Security, RoutePath.Resource].map(Mapper)}
        </div>

        <div className={cs.separator} />
        <div className={cs.group}>
          {widgets.value
            .filter((widget) => !widget.hidden)
            .map((widget) => (
              <Item
                key={widget.id}
                route={`/widget?${new URLSearchParams({ id: widget.id })}`}
                isActive={location.pathname === "/widget" &&
                  new URLSearchParams(location.search).get("id") === widget.id}
                collapsed={collapsed}
                label={<ResourceText text={widget.metadata.displayName} />}
                icon={<Icon iconName={(widget.icon as any) || "BiSolidWidget"} />}
              />
            ))}
        </div>

        {!!themesDirectAccess.length && (
          <>
            <div className={cs.separator} />
            <div className={cs.group}>
              {themesDirectAccess
                .toSorted((a, b) => {
                  const aName = getResourceText(a.metadata.displayName, i18n.language);
                  const bName = getResourceText(b.metadata.displayName, i18n.language);
                  return aName.localeCompare(bName, i18n.language);
                })
                .map((theme) => (
                  <Item
                    key={theme.id}
                    route={`/theme?${new URLSearchParams({ id: theme.id })}`}
                    isActive={location.pathname === "/theme" &&
                      new URLSearchParams(location.search).get("id") === theme.id}
                    collapsed={collapsed}
                    label={<ResourceText text={theme.metadata.displayName} />}
                    icon={<Icon iconName="BiSolidPalette" />}
                  />
                ))}
            </div>
          </>
        )}

        <div className={cs.separator} />
        <div className={cs.group}>{advanceGroup.map(Mapper)}</div>

        <div className={cs.separator} />
        <div className={cs.group}>{devGroup.map(Mapper)}</div>
      </div>

      <div className={cs.footer}>
        <SessionAvatar collapsed={collapsed} isActive={location.pathname === RoutePath.Extras} />
      </div>
    </div>
  );
});

interface ItemProps {
  route: string;
  isActive: boolean;
  collapsed: boolean;
  icon?: React.ReactNode;
  label: React.ReactNode;
  badge?: boolean;
}

const Item = ({ route, icon, label, isActive, collapsed, badge }: ItemProps) => {
  return (
    <Tooltip placement="right" title={collapsed ? label : null}>
      <NavLink
        to={route}
        className={cx(cs.item, {
          [cs.active!]: isActive,
        })}
      >
        <div className={cs.iconWrapper}>
          {icon}
          {badge && <span className={cs.badge} />}
        </div>
        <span className={cs.label}>{label}</span>
      </NavLink>
    </Tooltip>
  );
};

interface SessionAvatarProps {
  collapsed: boolean;
  isActive: boolean;
}

const SessionAvatar = ({ collapsed, isActive }: SessionAvatarProps) => {
  const { t } = useTranslation();

  return (
    <Tooltip placement="right" title={collapsed ? t("header.labels.extras") : null}>
      <NavLink
        to={RoutePath.Extras}
        className={cx(cs.sessionItem, {
          [cs.active!]: isActive,
        })}
      >
        <div className={cs.sessionAvatar}>
          <Icon iconName="LuSparkles" />
        </div>
        <div className={cs.sessionInfo}>
          <span className={cs.label}>KestrelUI</span>
          <span className={cs.email}>{t("header.labels.extras")}</span>
        </div>
      </NavLink>
    </Tooltip>
  );
};
