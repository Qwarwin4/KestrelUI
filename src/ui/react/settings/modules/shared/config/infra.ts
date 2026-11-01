import { app, path } from "@tauri-apps/api";

export const EnvConfig = {
  version: await app.getVersion(),
  // Seelen UI release whose resource format this build follows, resources declare
  // `appTargetVersion` against it. Bump when merging an upstream release
  seelenVersion: "2.8.11",
};

// "0.1.0-a" -> "0.1-a"
export function displayVersion(version: string): string {
  return version.replace(/^(\d+\.\d+)\.0(?=$|-)/, "$1");
}

export async function resolveDataPath(...sub: string[]) {
  return await path.join(await path.appDataDir(), ...sub);
}
