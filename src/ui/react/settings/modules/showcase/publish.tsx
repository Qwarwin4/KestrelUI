import { invoke, SeelenCommand } from "@seelen-ui/lib";
import type { GithubDeviceCode, GithubUser } from "@seelen-ui/lib/types";
import { signal } from "@preact/signals";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { Button, Flex, Input, Modal, Result } from "antd";
import { useState } from "preact/hooks";
import { useTranslation } from "react-i18next";

import cs from "./index.module.css";

export const githubUser = signal<GithubUser | null>(null);
invoke(SeelenCommand.ShowcaseGithubUser)
  .then((u) => (githubUser.value = u))
  .catch(console.error);

type Step = "closed" | "login" | "form" | "done";

export function PublishButton() {
  const [step, setStep] = useState<Step>("closed");
  const { t } = useTranslation();

  return (
    <>
      <Button type="primary" onClick={() => setStep(githubUser.value ? "form" : "login")}>
        <Icon iconName="LuUpload" />
        {t("showcase.publish")}
      </Button>
      <Modal
        open={step !== "closed"}
        onCancel={() => setStep("closed")}
        footer={null}
        destroyOnHidden
        centered
        width={560}
      >
        {step === "login" && <Login onDone={() => setStep("form")} />}
        {step === "form" && <PublishForm onDone={() => setStep("done")} />}
        {step === "done" && <Published onClose={() => setStep("closed")} />}
      </Modal>
    </>
  );
}

export function GithubAccount() {
  const user = githubUser.value;
  const { t } = useTranslation();
  if (!user) return null;

  async function logout() {
    await invoke(SeelenCommand.ShowcaseGithubLogout);
    githubUser.value = null;
  }

  return (
    <Flex align="center" gap={8} className={cs.account}>
      <img src={user.avatarUrl} alt="" />
      <span>@{user.login}</span>
      <Button type="text" size="small" onClick={logout}>
        {t("showcase.logout")}
      </Button>
    </Flex>
  );
}

function Login({ onDone }: { onDone: () => void }) {
  const [code, setCode] = useState<GithubDeviceCode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { t } = useTranslation();

  async function start() {
    setError(null);
    try {
      const c = await invoke(SeelenCommand.ShowcaseGithubLogin);
      setCode(c);
      navigator.clipboard.writeText(c.userCode).catch(() => {});
      githubUser.value = await invoke(SeelenCommand.ShowcaseGithubLoginWait);
      onDone();
    } catch (e) {
      setError(String(e));
      setCode(null);
    }
  }

  return (
    <Flex vertical gap={16} className={cs.publish}>
      <h2>{t("showcase.login_title")}</h2>
      <p>{t("showcase.login_body")}</p>
      {code
        ? (
          <>
            <div className={cs.deviceCode}>{code.userCode}</div>
            <p>{t("showcase.login_code_hint")}</p>
            <Button type="primary" href={code.verificationUri} target="_blank">
              <Icon iconName="LuGithub" />
              {t("showcase.login_open")}
            </Button>
          </>
        )
        : (
          <Button type="primary" onClick={start}>
            <Icon iconName="LuGithub" />
            {t("showcase.login_start")}
          </Button>
        )}
      {error && <p className={cs.error}>{error}</p>}
    </Flex>
  );
}

const lastPr = signal("");

function PublishForm({ onDone }: { onDone: () => void }) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");
  const [preview, setPreview] = useState<string | null>(null);
  const [busy, setBusy] = useState<"shot" | "send" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { t } = useTranslation();

  async function shoot() {
    setBusy("shot");
    setError(null);
    try {
      setPreview(await invoke(SeelenCommand.ShowcaseCapturePreview));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function send() {
    setBusy("send");
    setError(null);
    try {
      lastPr.value = await invoke(SeelenCommand.ShowcasePublish, {
        submission: { name, description, tags: tags.split(",") },
      });
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  const ready = name.trim() && description.trim() && preview;

  return (
    <Flex vertical gap={12} className={cs.publish}>
      <h2>{t("showcase.publish_title")}</h2>
      <p>{t("showcase.publish_body")}</p>
      <Input
        placeholder={t("showcase.field_name")}
        maxLength={48}
        value={name}
        onChange={(e) => setName((e.target as HTMLInputElement).value)}
      />
      <Input.TextArea
        placeholder={t("showcase.field_description")}
        maxLength={280}
        autoSize={{ minRows: 2, maxRows: 4 }}
        value={description}
        onChange={(e) => setDescription((e.target as HTMLTextAreaElement).value)}
      />
      <Input
        placeholder={t("showcase.field_tags")}
        value={tags}
        onChange={(e) => setTags((e.target as HTMLInputElement).value)}
      />
      <figure className={cs.shot}>
        {preview ? <img src={preview} alt="" /> : <span>{t("showcase.preview_hint")}</span>}
      </figure>
      <Flex gap={8} justify="flex-end">
        <Button onClick={shoot} loading={busy === "shot"} disabled={busy === "send"}>
          <Icon iconName="LuCamera" />
          {preview ? t("showcase.preview_retake") : t("showcase.preview_take")}
        </Button>
        <Button type="primary" onClick={send} loading={busy === "send"} disabled={!ready || !!busy}>
          {t("showcase.publish_send")}
        </Button>
      </Flex>
      {error && <p className={cs.error}>{error}</p>}
    </Flex>
  );
}

function Published({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <Result
      status="success"
      title={t("showcase.published_title")}
      subTitle={t("showcase.published_body")}
      extra={[
        <Button key="pr" type="primary" href={lastPr.value} target="_blank">
          {t("showcase.published_open")}
        </Button>,
        <Button key="close" onClick={onClose}>
          {t("showcase.published_close")}
        </Button>,
      ]}
    />
  );
}
