import type { ResourceId } from "@seelen-ui/lib/types";
import { Button, Flex, Input, InputNumber, Switch } from "antd";
import { useEffect, useRef, useState } from "preact/hooks";
import { useTranslation } from "react-i18next";

import { settings } from "../../../state/mod.ts";
import { SettingsGroup, SettingsOption } from "../../../components/SettingsBox/index.tsx";
import cs from "./neon.module.css";

export const NEON_THEME = "@kestrel/neon-moss" as ResourceId;
const DEFAULT = "#5ef0a0";

const presets = [
  "#5ef0a0",
  "#a3e635",
  "#22d3ee",
  "#60a5fa",
  "#a78bfa",
  "#f472b6",
  "#fb7185",
  "#fbbf24",
  "#fb923c",
  "#f5f5f4",
];

type Rgb = [number, number, number];
type Hsv = [number, number, number];

export function neonEnabled() {
  return settings.value.activeThemes.includes(NEON_THEME);
}

function themeVar(name: string): string | undefined {
  return settings.value.byTheme[NEON_THEME]?.[name];
}

function setThemeVar(name: string, value: string) {
  const s = settings.value;
  settings.value = {
    ...s,
    byTheme: { ...s.byTheme, [NEON_THEME]: { ...s.byTheme[NEON_THEME], [name]: value } },
  };
}

export function neonColor(): string {
  return themeVar("--neon-color") ?? DEFAULT;
}

function setEnabled(on: boolean) {
  const s = settings.value;
  const rest = s.activeThemes.filter((id) => id !== NEON_THEME);
  settings.value = { ...s, activeThemes: on ? [...rest, NEON_THEME] : rest };
}

export function NeonColor() {
  const { t } = useTranslation();
  const enabled = neonEnabled();
  const neon = neonColor();
  const iconFollows = (themeVar("--neon-icon-follow") ?? "1") !== "0";
  const icon = themeVar("--neon-icon-color") ?? neon;

  return (
    <>
      <SettingsGroup>
        <SettingsOption
          label={t("general.neon.title")}
          description={t("general.neon.description")}
          action={<Switch value={enabled} onChange={setEnabled} />}
        />
        {enabled && <ColorPicker value={neon} onChange={(hex) => setThemeVar("--neon-color", hex)} />}
      </SettingsGroup>

      {enabled && (
        <SettingsGroup>
          <SettingsOption
            label={t("general.neon.icon")}
            description={t("general.neon.icon_description")}
            action={
              <Flex align="center" gap={8}>
                <span className={cs.iconHint}>{t("general.neon.icon_follow")}</span>
                <Switch
                  value={iconFollows}
                  onChange={(on) => {
                    setThemeVar("--neon-icon-follow", on ? "1" : "0");
                    if (!on && !themeVar("--neon-icon-color")) setThemeVar("--neon-icon-color", neon);
                  }}
                />
              </Flex>
            }
          />
          {!iconFollows && (
            <ColorPicker value={icon} onChange={(hex) => setThemeVar("--neon-icon-color", hex)} />
          )}
        </SettingsGroup>
      )}
    </>
  );
}

function ColorPicker({ value, onChange }: { value: string; onChange: (hex: string) => void }) {
  const { t } = useTranslation();
  const [hsv, setHsv] = useState<Hsv>(() => rgbToHsv(hexToRgb(value) ?? hexToRgb(DEFAULT)!));
  const [hexText, setHexText] = useState(value);
  const rgb = hsvToRgb(hsv);

  // keep local state in sync when the value changes from outside (presets, discard)
  useEffect(() => {
    const parsed = hexToRgb(value);
    if (parsed && rgbToHex(hsvToRgb(hsv)) !== value.toLowerCase()) {
      setHsv(rgbToHsv(parsed));
    }
    setHexText(value);
  }, [value]);

  function commit(next: Hsv) {
    setHsv(next);
    onChange(rgbToHex(hsvToRgb(next)));
  }

  function commitRgb(i: number, v: number | null) {
    if (v === null) return;
    const next = [...rgb] as Rgb;
    next[i] = Math.max(0, Math.min(255, Math.round(v)));
    commit(rgbToHsv(next));
  }

  function commitHex(text: string) {
    setHexText(text);
    const parsed = hexToRgb(text);
    if (parsed) commit(rgbToHsv(parsed));
  }

  return (
    <div className={cs.picker}>
      <div className={cs.presets} role="list" aria-label={t("general.neon.presets")}>
        {presets.map((p) => (
          <button
            key={p}
            type="button"
            role="listitem"
            aria-label={p}
            className={cs.swatch}
            data-selected={p === value.toLowerCase()}
            style={{ backgroundColor: p }}
            onClick={() => onChange(p)}
          />
        ))}
      </div>

      <div className={cs.editor}>
        <div className={cs.planes}>
          <SvSquare hsv={hsv} onChange={commit} />
          <input
            type="range"
            min={0}
            max={359}
            value={Math.round(hsv[0])}
            aria-label={t("general.neon.hue")}
            className={cs.hue}
            onInput={(e) => commit([Number((e.target as HTMLInputElement).value), hsv[1], hsv[2]])}
          />
        </div>

        <div className={cs.fields}>
          <div className={cs.preview} style={{ backgroundColor: value }} />
          <label className={cs.field}>
            <span>HEX</span>
            <Input
              value={hexText}
              maxLength={7}
              onChange={(e) => commitHex((e.target as HTMLInputElement).value.trim())}
              onBlur={() => setHexText(value)}
            />
          </label>
          <div className={cs.rgb}>
            {(["R", "G", "B"] as const).map((ch, i) => (
              <label key={ch} className={cs.field}>
                <span>{ch}</span>
                <InputNumber min={0} max={255} value={rgb[i]} onChange={(v) => commitRgb(i, v)} />
              </label>
            ))}
          </div>
          <Button size="small" onClick={() => onChange(DEFAULT)}>
            {t("general.neon.reset")}
          </Button>
        </div>
      </div>
    </div>
  );
}

function SvSquare({ hsv, onChange }: { hsv: Hsv; onChange: (hsv: Hsv) => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [h, s, v] = hsv;

  function pick(e: PointerEvent) {
    const box = ref.current!.getBoundingClientRect();
    const x = Math.max(0, Math.min(1, (e.clientX - box.left) / box.width));
    const y = Math.max(0, Math.min(1, (e.clientY - box.top) / box.height));
    onChange([h, x, 1 - y]);
  }

  return (
    <div
      ref={ref}
      className={cs.square}
      style={{ backgroundColor: `hsl(${h} 100% 50%)` }}
      onPointerDown={(e) => {
        (e.currentTarget as HTMLDivElement).setPointerCapture(e.pointerId);
        pick(e);
      }}
      onPointerMove={(e) => {
        if ((e.currentTarget as HTMLDivElement).hasPointerCapture(e.pointerId)) pick(e);
      }}
    >
      <span className={cs.cursor} style={{ left: `${s * 100}%`, top: `${(1 - v) * 100}%` }} />
    </div>
  );
}

function hexToRgb(hex: string): Rgb | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex);
  if (!m) return null;
  let h = m[1]!;
  if (h.length === 3) h = [...h].map((c) => c + c).join("");
  const n = parseInt(h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbToHex([r, g, b]: Rgb): string {
  return "#" + [r, g, b].map((c) => c.toString(16).padStart(2, "0")).join("");
}

function rgbToHsv([r, g, b]: Rgb): Hsv {
  r /= 255;
  g /= 255;
  b /= 255;
  const max = Math.max(r, g, b);
  const d = max - Math.min(r, g, b);
  let h = 0;
  if (d) {
    if (max === r) h = ((g - b) / d) % 6;
    else if (max === g) h = (b - r) / d + 2;
    else h = (r - g) / d + 4;
  }
  return [(h * 60 + 360) % 360, max ? d / max : 0, max];
}

function hsvToRgb([h, s, v]: Hsv): Rgb {
  const f = (n: number) => {
    const k = (n + h / 60) % 6;
    return Math.round((v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255);
  };
  return [f(5), f(3), f(1)];
}
