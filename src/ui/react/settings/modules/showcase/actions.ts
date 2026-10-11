import { invoke, SeelenCommand } from "@seelen-ui/lib";
import { signal } from "@preact/signals";
import { message } from "antd";

import i18n from "../../i18n/index.ts";

export const installing = signal<string | null>(null);

export async function installFromShowcase(id: string) {
  if (installing.value) return;
  installing.value = id;
  try {
    await invoke(SeelenCommand.ShowcaseInstall, { id });
    message.success(i18n.t("showcase.installed"));
  } catch (e) {
    console.error(e);
    message.error(i18n.t("showcase.install_failed"));
  } finally {
    installing.value = null;
  }
}

export async function undoLastPreset() {
  try {
    const done = await invoke(SeelenCommand.ShowcaseUndo);
    if (done) message.success(i18n.t("showcase.undone"));
    else message.info(i18n.t("showcase.nothing_to_undo"));
  } catch (e) {
    console.error(e);
    message.error(String(e));
  }
}
