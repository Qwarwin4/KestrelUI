import { AssistantRole } from "@seelen-ui/lib/types";
import { Icon } from "libs/ui/react/components/Icon/index.tsx";
import { Button, Input, Modal } from "antd";
import { useEffect, useRef, useState } from "preact/hooks";
import { useTranslation } from "react-i18next";

import { Connection } from "./Connection.tsx";
import { chat, clearChat, config, configured, send, thinking, undoableIndex, undoAt } from "./state.ts";
import cs from "./index.module.css";

const SUGGESTIONS = ["neon", "dock", "glass", "tiling"] as const;

export function Assistant() {
  const { t } = useTranslation();
  const [setup, setSetup] = useState(false);

  return (
    <div className={cs.page}>
      <div className={cs.hero}>
        <div>
          <h1>{t("assistant.title")}</h1>
          <p>{t("assistant.subtitle")}</p>
        </div>
        {configured.value && (
          <div className={cs.heroActions}>
            <span className={cs.model}>
              {t(`assistant.providers.${config.value.provider}`)} · {config.value.model}
            </span>
            <Button onClick={() => setSetup(true)}>
              <Icon iconName="LuKeyRound" />
              {t("assistant.connection")}
            </Button>
          </div>
        )}
      </div>

      {configured.value ? <Chat /> : (
        <div className={cs.card}>
          <h2>{t("assistant.connect_title")}</h2>
          <p>{t("assistant.connect_description")}</p>
          <Connection />
        </div>
      )}

      <Modal
        open={setup}
        title={t("assistant.connection")}
        onCancel={() => setSetup(false)}
        footer={null}
        destroyOnHidden
        centered
        width={560}
      >
        <Connection onDone={() => setSetup(false)} />
      </Modal>
    </div>
  );
}

function Chat() {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const listRef = useRef<HTMLDivElement>(null);
  const messages = chat.value;

  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight, behavior: "smooth" });
  }, [messages.length, thinking.value]);

  function submit(value = text) {
    if (!value.trim() || thinking.value) return;
    setText("");
    send(value);
  }

  return (
    <div className={cs.chat}>
      <div ref={listRef} className={cs.messages}>
        {!messages.length && (
          <div className={cs.empty}>
            <Icon iconName="LuSparkles" size={28} />
            <p>{t("assistant.empty")}</p>
            <div className={cs.suggestions}>
              {SUGGESTIONS.map((s) => (
                <button key={s} type="button" onClick={() => submit(t(`assistant.suggestions.${s}`))}>
                  {t(`assistant.suggestions.${s}`)}
                </button>
              ))}
            </div>
          </div>
        )}

        {messages.map((m, i) => (
          <div
            key={i}
            className={cs.message}
            data-role={m.role}
            data-error={m.error ? "" : undefined}
          >
            <div className={cs.bubble}>{m.content}</div>
            {!!m.actions?.length && (
              <ul className={cs.actions}>
                {m.actions.map((a, j) => (
                  <li key={j} data-ok={a.ok}>
                    <Icon iconName={a.ok ? "LuCheck" : "LuTriangleAlert"} />
                    {a.summary || a.tool}
                  </li>
                ))}
              </ul>
            )}
            {m.changed && m.role === AssistantRole.Assistant && (
              m.undone
                ? <span className={cs.undone}>{t("assistant.undone")}</span>
                : i === undoableIndex.value && (
                  <Button size="small" type="text" onClick={() => undoAt(i)}>
                    <Icon iconName="LuUndo2" />
                    {t("assistant.undo")}
                  </Button>
                )
            )}
          </div>
        ))}

        {thinking.value && (
          <div className={cs.message} data-role={AssistantRole.Assistant}>
            <div className={cs.bubble}>
              <span className={cs.dots}>
                <i />
                <i />
                <i />
              </span>
            </div>
          </div>
        )}
      </div>

      <div className={cs.composer}>
        <Input.TextArea
          value={text}
          autoSize={{ minRows: 1, maxRows: 6 }}
          placeholder={t("assistant.placeholder")}
          onChange={(e) => setText((e.target as HTMLTextAreaElement).value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              submit();
            }
          }}
        />
        <Button type="primary" disabled={!text.trim() || thinking.value} onClick={() => submit()}>
          <Icon iconName="LuSend" />
        </Button>
        <Button
          type="text"
          disabled={!messages.length || thinking.value}
          onClick={clearChat}
          title={t("assistant.clear")}
        >
          <Icon iconName="LuTrash2" />
        </Button>
      </div>
    </div>
  );
}
