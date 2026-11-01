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
  }, [neon, color]);

  return (
    <ConfigProvider
      componentSize="small"
      theme={{
        token: neon ? NEON_TOKENS(color) : { colorPrimary: color },
        algorithm: isDarkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
      }}
    >
      <Routing />
      <ThumbnailGeneratorModal />
      <WelcomeModal />
    </ConfigProvider>
  );
}

const NEON_TOKENS = (color: string) => ({
  colorPrimary: color,
  colorBgBase: "#08100c",
  colorBgContainer: "#0c1611",
  colorBgElevated: "#0f1a14",
  colorBorder: "rgba(125, 255, 178, 0.16)",
  colorBorderSecondary: "rgba(125, 255, 178, 0.1)",
  borderRadius: 10,
  fontFamily: '"Segoe UI Variable Text", "Segoe UI", sans-serif',
});
