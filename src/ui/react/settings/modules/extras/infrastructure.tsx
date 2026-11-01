import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { Button, Flex, Select, Switch, Tooltip } from "antd";
import { useTranslation } from "react-i18next";

import { displayVersion, EnvConfig } from "../shared/config/infra.ts";
import cs from "./infra.module.css";

import {
  getStreamingMode,
  getUpdaterSettings,
  patchUpdaterSettings,
  setStreamingMode,
} from "./application.ts";

import { SettingsGroup, SettingsOption, SettingsSubGroup } from "../../components/SettingsBox/index.tsx";
import { UpdateChannel } from "@seelen-ui/lib/types";

const [isDevMode, isMsixBuild, isFixed] = await Promise.all([
  invoke(SeelenCommand.IsDevMode),
  invoke(SeelenCommand.IsAppxPackage),
  invoke(SeelenCommand.HasFixedRuntime),
]);

export function Information() {
  const streamingMode = getStreamingMode();
  const updaterSettings = getUpdaterSettings();

  const { t } = useTranslation();

  function onToggleStreamingMode(value: boolean) {
    setStreamingMode(value);
  }

  function onChangeUpdateChannel(channel: UpdateChannel) {
    patchUpdaterSettings({ channel });
  }

  return (
    <div className={cs.info}>
      <SettingsGroup>
        <SettingsSubGroup label="KestrelUI">
          <SettingsOption
            label={t("extras.version")}
            description={isFixed ? t("extras.version_fixed") : false}
            action={
              <span className={cs.version}>
                v{displayVersion(EnvConfig.version)} {isDevMode && "(dev)"} {isMsixBuild && "(msix)"} {isFixed && "(fixed)"}
              </span>
            }
          />

          {!isFixed && !isMsixBuild && (
            <SettingsOption
              label={t("update.channel")}
              action={
                <Select
                  value={updaterSettings.channel}
                  onChange={onChangeUpdateChannel}
                  options={Object.values(UpdateChannel).map((c) => ({
                    value: c,
                    label: c,
                  }))}
                />
              }
            />
          )}

          <SettingsOption
            label={t("extras.license")}
            description={t("extras.license_notice")}
            action={
              <Flex gap={4}>
                <Button type="link" href="https://github.com/Qwarwin4/KestrelUI" target="_blank">
                  {t("extras.source_code")}
                </Button>
                <Button type="link" href="https://www.gnu.org/licenses/agpl-3.0.html" target="_blank">
                  AGPL-3.0
                </Button>
              </Flex>
            }
          />
        </SettingsSubGroup>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsOption
          label={t("extras.streaming_mode")}
          description={t("extras.streaming_mode_description")}
          action={<Switch value={streamingMode} onChange={onToggleStreamingMode} />}
        />
      </SettingsGroup>

      <SettingsGroup>
        <SettingsOption>
          <b style={{ display: "flex", alignItems: "center", gap: "4px" }}>
            {t("extras.clear_icons")}
            <Tooltip title={t("extras.clear_icons_tooltip")}>
              <Icon iconName="LuCircleHelp" />
            </Tooltip>
          </b>
          <Button
            type="dashed"
            danger
            onClick={() => invoke(SeelenCommand.StateDeleteCachedIcons)}
            style={{ width: "50px" }}
          >
            <Icon iconName="IoReload" size={12} />
          </Button>
        </SettingsOption>
      </SettingsGroup>

      <SettingsGroup>
        <SettingsOption>
          <b>{t("extras.relaunch")}</b>
          <Button type="dashed" onClick={() => invoke(SeelenCommand.AppRestart)} style={{ width: "50px" }}>
            <Icon iconName="IoReload" size={12} />
          </Button>
        </SettingsOption>
        <SettingsOption>
          <b>{t("extras.exit")}</b>
          <Button type="dashed" danger onClick={() => invoke(SeelenCommand.AppExit)} style={{ width: "50px" }}>
            <Icon iconName="IoClose" />
          </Button>
        </SettingsOption>
      </SettingsGroup>
    </div>
  );
}
