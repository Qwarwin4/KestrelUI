// Cleanup utilities

import fs from "fs";
import path from "path";
import { DIST_DIR, ICONS_DIR } from "../config.ts";

/**
 * Wipes the dist directory. Only runs on production builds — dev builds skip this so
 * incremental copies can reuse existing artifacts. The extracted icons stay: they only
 * depend on the react-icons version and take ~20s to regenerate (see steps/icons.ts).
 */
export function cleanDist(): void {
  console.info("Cleaning dist...");
  fs.mkdirSync(DIST_DIR, { recursive: true });
  const keep = path.resolve(ICONS_DIR);
  for (const entry of fs.readdirSync(DIST_DIR)) {
    const full = path.resolve(DIST_DIR, entry);
    if (full !== keep) {
      fs.rmSync(full, { recursive: true, force: true });
    }
  }
}
