import { Button, Modal } from "antd";
import { z } from "zod";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { persistentSignal } from "libs/ui/react/utils/PersistentSignal";

import cs from "./index.module.css";

const WelcomeStateSchema = z.object({
  welcomeShown: z.boolean().default(false),
});

const WelcomeState = await persistentSignal(
  "welcomeModal",
  WelcomeStateSchema.parse({}),
  WelcomeStateSchema,
);

/** shown once, on the first start */
export const WelcomeModal = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const close = (to?: string) => {
    WelcomeState.value = { welcomeShown: true };
    if (to) navigate(to);
  };

  return (
    <Modal
      className={cs.welcome}
      open={!WelcomeState.value.welcomeShown}
      centered
      closable={false}
      footer={null}
      width={460}
    >
      <div className={cs.body}>
        <span className={cs.logo} aria-hidden="true">
          <span style={{ maskImage: "url(./logo-mark.svg)" }} />
        </span>
        <h2>{t("welcome.title")}</h2>
        <p>{t("welcome.message")}</p>
        <div className={cs.actions}>
          <Button size="large" onClick={() => close("/general")}>
            {t("welcome.customize")}
          </Button>
          <Button size="large" type="primary" onClick={() => close()}>
            {t("welcome.ok")}
          </Button>
        </div>
      </div>
    </Modal>
  );
};
