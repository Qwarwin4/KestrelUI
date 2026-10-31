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

  return (
    <ConfigProvider
      componentSize="small"
      theme={{
        token: {
          colorPrimary: color,
        },
        algorithm: isDarkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
      }}
    >
      <Routing />
      <ThumbnailGeneratorModal />
      <WelcomeModal />
    </ConfigProvider>
  );
}
