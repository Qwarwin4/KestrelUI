<script lang="ts">
  import { invoke, SeelenCommand } from "@seelen-ui/lib";
  import { Icon } from "libs/ui/svelte/components/Icon";
  import { state } from "../state.svelte";
  import { t } from "../i18n";
  import { brightnessIcon } from "libs/ui/utils";
  import { throttle } from "lodash";

  const setBrightnessThrottled = throttle((id: string, brightness: number) => {
    invoke(SeelenCommand.SetMonitorBrightness, { id, brightness });
  }, 100);

  // one tap for every screen at once
  const presets = [
    { key: "night", value: 20, icon: "TbMoon" },
    { key: "reading", value: 50, icon: "TbBook" },
    { key: "day", value: 80, icon: "TbSun" },
    { key: "max", value: 100, icon: "TbSunHigh" },
  ] as const;

  const controllable = $derived(state.monitors.filter((m) => m.brightness != null));
  const current = $derived(
    controllable.length && controllable.every((m) => m.brightness === controllable[0]!.brightness)
      ? controllable[0]!.brightness
      : null,
  );

  function applyPreset(value: number) {
    for (const monitor of controllable) {
      invoke(SeelenCommand.SetMonitorBrightness, { id: monitor.id, brightness: value });
    }
  }
</script>

{#if controllable.length}
  <div class="brightness-presets">
    {#each presets as preset (preset.key)}
      <button
        data-skin={current === preset.value ? "solid" : "default"}
        class="brightness-preset"
        onclick={() => applyPreset(preset.value)}
      >
        <Icon iconName={preset.icon} />
        <span>{$t(`brightness_presets.${preset.key}`)}</span>
      </button>
    {/each}
  </div>
{/if}

{#each controllable as monitor (monitor.id)}
  <span class="quick-settings-label">{monitor.name}</span>
  <div class="quick-settings-item">
    <button data-skin="transparent">
      <Icon iconName={brightnessIcon(monitor.brightness!)} />
    </button>
    <input
      type="range"
      data-skin="flat"
      value={monitor.brightness}
      oninput={(e) => {
        setBrightnessThrottled(monitor.id, Number(e.currentTarget.value));
      }}
      min={0}
      max={100}
    />
  </div>
{/each}
