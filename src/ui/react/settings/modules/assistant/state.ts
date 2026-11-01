import { invoke, SeelenCommand } from "@seelen-ui/lib";
import type { AssistantAction, AssistantMessage, AssistantSettings } from "@seelen-ui/lib/types";
import { AssistantRole } from "@seelen-ui/lib/types";
import { computed, signal } from "@preact/signals";

import { saveSettings, settings } from "../../state/mod.ts";

export interface ChatEntry {
  role: AssistantRole;
  content: string;
  actions?: AssistantAction[];
  changed?: boolean;
  undone?: boolean;
  error?: boolean;
}

// module level, so the chat survives switching pages while the window is open
export const chat = signal<ChatEntry[]>([]);
export const thinking = signal(false);

export const config = computed(() => settings.value.assistant);
export const configured = computed(() => {
  const c = config.value;
  return !!c.model.trim() && (c.provider !== "Custom" || !!c.baseUrl.trim());
});

export async function saveConfig(next: AssistantSettings) {
  settings.value = { ...settings.value, assistant: next };
  await saveSettings();
}

/** only the newest change can be undone, snapshots are a stack */
export const undoableIndex = computed(() => chat.value.findLastIndex((m) => m.changed && !m.undone));

export async function send(text: string) {
  const content = text.trim();
  if (!content || thinking.value) return;

  chat.value = [...chat.value, { role: AssistantRole.User, content }];
  thinking.value = true;

  const history: AssistantMessage[] = chat.value
    .filter((m) => !m.error)
    .map((m) => ({ role: m.role, content: m.content }));

  try {
    const reply = await invoke(SeelenCommand.AssistantChat, { messages: history });
    chat.value = [
      ...chat.value,
      {
        role: AssistantRole.Assistant,
        content: reply.content,
        actions: reply.actions,
        changed: reply.changedSettings,
      },
    ];
  } catch (e) {
    chat.value = [...chat.value, { role: AssistantRole.Assistant, content: String(e), error: true }];
  } finally {
    thinking.value = false;
  }
}

export async function undoAt(index: number) {
  const done = await invoke(SeelenCommand.ShowcaseUndo);
  if (!done) return;
  chat.value = chat.value.map((m, i) => (i === index ? { ...m, undone: true } : m));
}

export function clearChat() {
  if (!thinking.value) chat.value = [];
}
