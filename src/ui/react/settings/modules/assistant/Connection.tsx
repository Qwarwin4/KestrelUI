import { invoke, SeelenCommand } from "@seelen-ui/lib";
import type { AssistantSettings } from "@seelen-ui/lib/types";
import { AssistantProvider } from "@seelen-ui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { AutoComplete, Button, Flex, Input, message, Select } from "antd";
import { useEffect, useState } from "preact/hooks";
import { useTranslation } from "react-i18next";

import { config, saveConfig } from "./state.ts";
import cs from "./index.module.css";

const KEY_PAGES: Partial<Record<AssistantProvider, string>> = {
  [AssistantProvider.OpenAI]: "https://platform.openai.com/api-keys",
  [AssistantProvider.Anthropic]: "https://console.anthropic.com/settings/keys",
  [AssistantProvider.OpenRouter]: "https://openrouter.ai/keys",
};

const MODEL_HINTS: Record<AssistantProvider, string> = {
  [AssistantProvider.OpenAI]: "gpt-4.1-mini",
  [AssistantProvider.Anthropic]: "claude-haiku-4-5",
  [AssistantProvider.OpenRouter]: "openai/gpt-4.1-mini",
  [AssistantProvider.Custom]: "local-model",
};

export function Connection({ onDone }: { onDone?: () => void }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<AssistantSettings>(config.value);
  const [key, setKey] = useState("");
  const [hasKey, setHasKey] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [saving, setSaving] = useState(false);

  const provider = draft.provider;
  const keyOptional = provider === AssistantProvider.Custom;

  useEffect(() => {
    setKey("");
    setModels([]);
    invoke(SeelenCommand.AssistantHasKey, { provider }).then(setHasKey).catch(() => setHasKey(false));
  }, [provider]);

  async function loadModels() {
    setLoadingModels(true);
    try {
      // the list is read with the saved provider, so save what is on screen first
      await persist(false);
      setModels(await invoke(SeelenCommand.AssistantListModels));
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoadingModels(false);
    }
  }

  async function persist(close: boolean) {
    setSaving(true);
    try {
      if (key.trim()) {
        await invoke(SeelenCommand.AssistantSetKey, { provider, key: key.trim() });
        setKey("");
        setHasKey(true);
      }
      await saveConfig({ ...draft, model: draft.model.trim(), baseUrl: draft.baseUrl.trim() });
      if (close) {
        message.success(t("assistant.saved"));
        onDone?.();
      }
    } finally {
      setSaving(false);
    }
  }

  async function forgetKey() {
    await invoke(SeelenCommand.AssistantSetKey, { provider, key: null });
    setHasKey(false);
  }

  return (
    <div className={cs.connection}>
      <label className={cs.field}>
        <span>{t("assistant.provider")}</span>
        <Select
          value={provider}
          onChange={(p) => setDraft({ ...draft, provider: p, model: "" })}
          options={Object.values(AssistantProvider).map((p) => ({
            value: p,
            label: t(`assistant.providers.${p}`),
          }))}
        />
      </label>

      {provider === AssistantProvider.Custom && (
        <label className={cs.field}>
          <span>{t("assistant.base_url")}</span>
          <Input
            value={draft.baseUrl}
            placeholder="http://localhost:1234/v1"
            onChange={(e) => setDraft({ ...draft, baseUrl: (e.target as HTMLInputElement).value })}
          />
        </label>
      )}

      <label className={cs.field}>
        <span>
          {t("assistant.api_key")}
          {keyOptional && ` (${t("assistant.optional")})`}
        </span>
        <Flex gap={8}>
          <Input.Password
            value={key}
            placeholder={hasKey ? t("assistant.key_saved") : "sk-..."}
            onChange={(e) => setKey((e.target as HTMLInputElement).value)}
          />
          {hasKey && (
            <Button onClick={forgetKey} title={t("assistant.forget_key")}>
              <Icon iconName="LuTrash2" />
            </Button>
          )}
        </Flex>
        <small>
          {t("assistant.key_vault")}
          {KEY_PAGES[provider] && (
            <>
              {" "}
              <a href={KEY_PAGES[provider]} target="_blank">{t("assistant.get_key")}</a>
            </>
          )}
        </small>
      </label>

      <label className={cs.field}>
        <span>{t("assistant.model")}</span>
        <Flex gap={8}>
          <AutoComplete
            className={cs.grow}
            value={draft.model}
            placeholder={MODEL_HINTS[provider]}
            options={models.map((m) => ({ value: m }))}
            filterOption={(input, option) => String(option?.value ?? "").toLowerCase().includes(input.toLowerCase())}
            onChange={(model) => setDraft({ ...draft, model })}
          />
          <Button onClick={loadModels} loading={loadingModels} title={t("assistant.load_models")}>
            <Icon iconName="LuRefreshCw" />
          </Button>
        </Flex>
      </label>

      <Flex justify="end">
        <Button
          type="primary"
          loading={saving}
          onClick={() => persist(true).catch((e) => message.error(String(e)))}
        >
          {t("assistant.save")}
        </Button>
      </Flex>
    </div>
  );
}
