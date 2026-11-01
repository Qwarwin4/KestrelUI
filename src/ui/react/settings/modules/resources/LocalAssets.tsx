import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { ResourceKind } from "@seelen-ui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import type { IconName } from "libs/ui/icons.ts";
import { Button, message, Tooltip } from "antd";
import { useState } from "preact/hooks";
import { useTranslation } from "react-i18next";

import type { ShowcaseKind } from "../showcase/registry.ts";
import cs from "./infra.module.css";

const KINDS: Partial<Record<ShowcaseKind, ResourceKind>> = {
  theme: ResourceKind.Theme,
  widget: ResourceKind.Widget,
  plugin: ResourceKind.Plugin,
  iconPack: ResourceKind.IconPack,
  wallpaper: ResourceKind.Wallpaper,
};

/** Resources from disk: import a folder or a file, an icon pack from plain pictures,
 * or manage the user folder of this kind by hand. */
export function LocalAssets({ discover }: { discover: ShowcaseKind }) {
  const { t } = useTranslation();
  const [busy, setBusy] = useState(false);
  const kind = KINDS[discover];
  if (!kind) {
    return null;
  }

  async function run(action: () => Promise<unknown>, done?: string) {
    setBusy(true);
    try {
      const result = await action();
      if (done && result) {
        message.success(done);
      }
    } catch (e) {
      message.error(String(e));
    } finally {
      setBusy(false);
    }
  }

  const button = (icon: IconName, label: string, onClick: () => void, primary = false) => (
    <Tooltip title={label}>
      <Button type={primary ? "primary" : "default"} disabled={busy} onClick={onClick} aria-label={label}>
        <Icon iconName={icon} />
      </Button>
    </Tooltip>
  );

  // icon packs and wallpapers keep their pictures next to the metadata, so only folders
  // and .slu bundles make sense for them; wallpaper pictures have their own import button
  const fileLabel = kind === ResourceKind.IconPack ? t("resources.local.import_slu") : t("resources.local.import_file");

  return (
    <div className={cs.localAssets}>
      <div>
        <b>{t("resources.local.title")}</b>
        <span>
          {kind === ResourceKind.IconPack ? t("resources.local.hint_icons") : t("resources.local.hint")}
        </span>
      </div>
      <div className={cs.localActions}>
        {kind === ResourceKind.IconPack &&
          button("LuImagePlus", t("resources.local.from_images"), () =>
            run(
              () => invoke(SeelenCommand.CreateIconPackFromImages),
              t("resources.local.imported"),
            ), true)}
        {kind !== ResourceKind.Wallpaper &&
          button("LuFolderInput", t("resources.local.import_folder"), () =>
            run(
              () => invoke(SeelenCommand.ImportLocalResource, { kind, folder: true }),
              t("resources.local.imported"),
            ))}
        {kind !== ResourceKind.Wallpaper &&
          button("LuFileInput", fileLabel, () =>
            run(
              () => invoke(SeelenCommand.ImportLocalResource, { kind, folder: false }),
              t("resources.local.imported"),
            ))}
        {button("LuFolderOpen", t("resources.local.open_folder"), () =>
          run(() =>
            invoke(SeelenCommand.OpenUserResourceFolder, { kind })
          ))}
        {button("LuRefreshCw", t("resources.local.reload"), () =>
          run(
            () => invoke(SeelenCommand.ReloadUserResources, { kind }).then(() => true),
            t("resources.local.reloaded"),
          ))}
      </div>
    </div>
  );
}
