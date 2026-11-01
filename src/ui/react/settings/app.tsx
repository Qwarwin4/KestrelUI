import { useDarkMode } from "libs/ui/react/utils/styling.ts";
import { ConfigProvider, theme } from "antd";
import { useEffect } from "react";
import { Color, Widget } from "@seelen-ui/lib";

import { Routing } from "./router.tsx";
import { ThumbnailGeneratorModal } from "./components/ThumbnailGeneratorModal/index.tsx";
import { WelcomeModal } from "./components/WelcomeModal/infra.tsx";
import { uiColors } from "./state/system.ts";
import { neonColor, neonEnabled } from "./modules/general/infra/NeonColor.tsx";

export function App() {
  const neon = neonEnabled();
  const isDarkMode = useDarkMode() || neon;

  useEffect(() => {
    setTimeout(() => {
      let splashscreen = document.getElementById("splashscreen");
      splashscreen?.classList.add("vanish");
      setTimeout(() => splashscreen?.classList.add("hidden"), 300);
    }, 300);

    Widget.self.ready();
  }, []);

  const color = neon
    ? neonColor()
    : new Color(isDarkMode ? uiColors.value.accent_light : uiColors.value.accent_dark).toHexString();

  // the variant A styles hang off this attribute, see styles/neon.css
  useEffect(() => {
    const root = document.documentElement;
    root.toggleAttribute("data-neon", neon);
    root.style.setProperty("--neon", color);
    // dark text on light accents (the neon), white on dark ones (most Windows accents)
    root.style.setProperty("--neon-ink", luminance(color) > 0.45 ? "#04130b" : "#ffffff");
    // the splash screen in index.html paints itself with this before the app loads
    try {
      localStorage.setItem("kestrel.splash-color", color);
    } catch {
      // first start keeps the default splash
    }
  }, [neon, color]);

  return (
    <ConfigProvider
      componentSize="small"
      theme={{
        token: neon ? NEON_TOKENS(color) : { colorPrimary: color, borderRadius: 10, fontFamily: FONT },
        algorithm: isDarkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
      }}
    >
      <Routing />
      <ThumbnailGeneratorModal />
      <WelcomeModal />
    </ConfigProvider>
  );
}

const FONT = '"Manrope Variable", "Segoe UI Variable Text", "Segoe UI", sans-serif';

// antd parses its tokens itself, so they can't use the oklch(from ...) colors of neon.css
const NEON_TOKENS = (color: string) => ({
  colorPrimary: color,
  colorBgBase: tinted(color, 0.16),
  colorBgContainer: tinted(color, 0.19),
  colorBgElevated: tinted(color, 0.21),
  colorBorder: withAlpha(color, 0.16),
  colorBorderSecondary: withAlpha(color, 0.1),
  borderRadius: 10,
  fontFamily: FONT,
});

function rgbOf(hex: string): [number, number, number] {
  const n = parseInt(hex.replace("#", "").slice(0, 6), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function withAlpha(hex: string, alpha: number): string {
  const [r, g, b] = rgbOf(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** a dark grey at OKLab lightness `l` with the hue of `hex` and a tenth of its chroma,
 * the same rule as the greys in styles/neon.css: a white neon gives a neutral grey */
function tinted(hex: string, l: number): string {
  const lin = rgbOf(hex).map((c) => {
    const v = c / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  const [r, g, b] = lin;
  const l1 = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m1 = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s1 = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  const A = (1.9779984951 * l1 - 2.428592205 * m1 + 0.4505937099 * s1) * 0.1;
  const B = (0.0259040371 * l1 + 0.7827717662 * m1 - 0.808675766 * s1) * 0.1;

  const l2 = (l + 0.3963377774 * A + 0.2158037573 * B) ** 3;
  const m2 = (l - 0.1055613458 * A - 0.0638541728 * B) ** 3;
  const s2 = (l - 0.0894841775 * A - 1.291485548 * B) ** 3;
  const out = [
    4.0767416621 * l2 - 3.3077115913 * m2 + 0.2309699292 * s2,
    -1.2684380046 * l2 + 2.6097574011 * m2 - 0.3413193965 * s2,
    -0.0041960863 * l2 - 0.7034186147 * m2 + 1.707614701 * s2,
  ].map((v) => {
    const c = Math.min(1, Math.max(0, v));
    const srgb = c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055;
    return Math.round(srgb * 255).toString(16).padStart(2, "0");
  });
  return `#${out.join("")}`;
}

/** relative luminance of a #rrggbb color, 0..1 */
function luminance(hex: string): number {
  const channel = (c: number) => {
    const v = c / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const [r, g, b] = rgbOf(hex);
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}
