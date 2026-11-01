import type { UpdaterSettings } from "@seelen-ui/lib/types";

import { settings } from "../../state/mod";

/**
 * Gets the streaming mode setting
 */
export function getStreamingMode(): boolean {
  return settings.value.streamingMode;
}

/**
 * Sets the streaming mode setting
 */
export function setStreamingMode(streamingMode: boolean) {
  settings.value = {
    ...settings.value,
    streamingMode,
  };
}

/**
 * Gets the updater settings
 */
export function getUpdaterSettings(): UpdaterSettings {
  return settings.value.updater;
}

/**
 * Patches the updater settings with partial updates
 */
export function patchUpdaterSettings(patch: Partial<UpdaterSettings>) {
  settings.value = {
    ...settings.value,
    updater: {
      ...settings.value.updater,
      ...patch,
    },
  };
}
