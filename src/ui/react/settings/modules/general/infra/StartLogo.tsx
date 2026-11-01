import { invoke, SeelenCommand } from "@seelen-ui/lib";
import type { IconPackEntry } from "@seelen-ui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { message, Tooltip } from "antd";
import type { ComponentChildren } from "preact";
import { useMemo, useRef, useState } from "preact/hooks";
import { useTranslation } from "react-i18next";

import { SettingsGroup, SettingsOption } from "../../../components/SettingsBox/index.tsx";
import { type PixelSprite, START_PRESETS } from "./startPresets.ts";
import cs from "./neon.module.css";

// the icon the dock's start button plugin renders, see static/plugins/weg_start_menu
const START_ICON: IconPackEntry = {
  type: "custom",
  key: "@seelen/weg::start-menu",
  icon: { base: null, light: null, dark: null, mask: null, isAproximatelySquare: true },
};
const SIZE = 512;
// which tile is highlighted; the icon itself lives in the user icon pack
const PICKED_KEY = "kestrel.start-logo";

function drawSprite(sprite: PixelSprite, size: number): HTMLCanvasElement {
  const h = sprite.rows.length;
  const w = Math.max(...sprite.rows.map((r) => r.length));
  const scale = Math.floor(size / Math.max(w, h));
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d")!;
  const ox = Math.floor((size - w * scale) / 2);
  const oy = Math.floor((size - h * scale) / 2);
  sprite.rows.forEach((row, y) => {
    [...row].forEach((ch, x) => {
      const color = sprite.colors[ch];
      if (!color) return;
      ctx.fillStyle = color;
      ctx.fillRect(ox + x * scale, oy + y * scale, scale, scale);
    });
  });
  return canvas;
}

async function imageFileToPng(file: File): Promise<string> {
  const url = URL.createObjectURL(file);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = SIZE;
    canvas.height = SIZE;
    const ctx = canvas.getContext("2d")!;
    // small pictures are most likely pixel art, keep their edges sharp
    ctx.imageSmoothingEnabled = Math.max(img.naturalWidth, img.naturalHeight) > 96;
    const k = Math.min(SIZE / img.naturalWidth, SIZE / img.naturalHeight);
    const w = img.naturalWidth * k;
    const h = img.naturalHeight * k;
    ctx.drawImage(img, (SIZE - w) / 2, (SIZE - h) / 2, w, h);
    return canvas.toDataURL("image/png").split(",")[1]!;
  } finally {
    URL.revokeObjectURL(url);
  }
}

function readPicked(): string {
  try {
    return localStorage.getItem(PICKED_KEY) ?? "default";
  } catch {
    return "default";
  }
}

function savePicked(id: string) {
  try {
    localStorage.setItem(PICKED_KEY, id);
  } catch {
    // only the highlight is lost
  }
}

export function StartLogo() {
  const { t } = useTranslation();
  const [picked, setPicked] = useState(readPicked);
  const [busy, setBusy] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);

  const previews = useMemo(
    () => Object.fromEntries(START_PRESETS.map((s) => [s.id, drawSprite(s, 80).toDataURL()])),
    [],
  );

  async function apply(id: string, iconBase64: string | null) {
    setBusy(true);
    try {
      if (iconBase64) {
        await invoke(SeelenCommand.RegisterUserCustomAppIcon, { iconBase64, entry: START_ICON });
      } else {
        await invoke(SeelenCommand.DeleteUserCustomAppIcon, { entry: START_ICON });
      }
      setPicked(id);
      savePicked(id);
    } catch (e) {
      message.error(String(e));
    } finally {
      setBusy(false);
    }
  }

  function applySprite(sprite: PixelSprite) {
    const png = drawSprite(sprite, SIZE).toDataURL("image/png").split(",")[1]!;
    apply(sprite.id, png);
  }

  async function onFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    try {
      apply("custom", await imageFileToPng(file));
    } catch {
      message.error(t("general.start_logo.bad_file"));
    }
  }

  const tile = (id: string, label: string, content: ComponentChildren, onClick: () => void) => (
    <Tooltip key={id} title={label}>
      <button
        type="button"
        className={cs.logoTile}
        data-selected={picked === id}
        disabled={busy}
        aria-label={label}
        onClick={onClick}
      >
        {content}
      </button>
    </Tooltip>
  );

  const group = (name: "girls" | "other") => (
    <div className={cs.logoGroup}>
      <span>{t(`general.start_logo.${name}`)}</span>
      <div className={cs.logoGrid}>
        {START_PRESETS.filter((s) => s.group === name).map((s) =>
          tile(
            s.id,
            t(`general.start_logo.presets.${s.id}`),
            <img src={previews[s.id]} alt="" />,
            () => applySprite(s),
          )
        )}
      </div>
    </div>
  );

  return (
    <SettingsGroup>
      <SettingsOption
        label={t("general.start_logo.title")}
        description={t("general.start_logo.description")}
      />
      <div className={cs.logoGroup}>
        <div className={cs.logoGrid}>
          {tile(
            "default",
            t("general.start_logo.default"),
            <span className={cs.logoMark} style={{ maskImage: "url(./logo-mark.svg)" }} />,
            () => apply("default", null),
          )}
          {tile(
            "custom",
            t("general.start_logo.custom"),
            <Icon iconName="LuImagePlus" size={22} />,
            () => fileRef.current?.click(),
          )}
        </div>
      </div>
      {group("girls")}
      {group("other")}
      <input ref={fileRef} type="file" accept="image/*" hidden onChange={onFile} />
    </SettingsGroup>
  );
}
