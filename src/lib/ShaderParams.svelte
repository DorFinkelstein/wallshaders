<script lang="ts">
  import Icon from "./Icon.svelte";
  import { formatValue, fromHex, toHex, type Param } from "./params";

  interface Props {
    params: Param[];
    /** Some value differs from the saved file. */
    changed: boolean;
    onchange: (param: Param, value: Param["value"]) => void;
    onreset: () => void;
  }
  let { params, changed, onchange, onreset }: Props = $props();

  const KEY = "wallshaders.paramsOpen";
  function readOpen(): boolean {
    try {
      return localStorage.getItem(KEY) !== "0";
    } catch {
      return true;
    }
  }
  let open = $state(readOpen());
  function toggle() {
    open = !open;
    try {
      localStorage.setItem(KEY, open ? "1" : "0");
    } catch {
      // Not saved; the panel just opens next time.
    }
  }

  function shown(p: Param): string {
    return p.kind === "color" ? "" : formatValue(p, p.value).replace(/(\.\d*?)0+$/, "$1").replace(/\.$/, "");
  }
</script>

<section class="params" aria-label="Shader settings">
  <header>
    <button class="title" onclick={toggle} aria-expanded={open}>
      <Icon name="sliders" size={15} />
      <span>Shader settings</span>
      <span class="count">{params.length}</span>
      <span class="chev" class:open><Icon name="chevron" size={13} /></span>
    </button>
    {#if open && changed}
      <button class="reset" onclick={onreset} title="Go back to the values in the saved file">Reset</button>
    {/if}
  </header>
  {#if open}
    <div class="grid">
      {#each params as p (p.name)}
        <label class="control" title={p.name}>
          <span class="label">{p.label}</span>
          {#if p.kind === "color"}
            <span class="color-row">
              <input
                type="color"
                value={toHex(p.value as [number, number, number])}
                oninput={(e) => onchange(p, fromHex(e.currentTarget.value))}
              />
              <span class="value">{toHex(p.value as [number, number, number])}</span>
            </span>
          {:else}
            <span class="slider-row">
              <input
                type="range"
                min={p.min}
                max={p.max}
                step={p.step}
                value={p.value as number}
                oninput={(e) => onchange(p, Number(e.currentTarget.value))}
              />
              <span class="value">{shown(p)}</span>
            </span>
          {/if}
        </label>
      {/each}
    </div>
  {/if}
</section>

<style>
  .params {
    flex: none;
    margin: 0 18px 12px;
    border-radius: var(--radius);
    background: var(--bg-2);
    border: 1px solid var(--line);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 6px 0 4px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 8px;
    color: var(--text-dim);
    font-weight: 600;
    font-size: 12.5px;
  }
  .title:hover {
    color: var(--text);
  }
  .count {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 5px;
    background: var(--bg-3);
    font-size: 11px;
    text-align: center;
  }
  .chev {
    display: grid;
    transform: rotate(-90deg);
    transition: transform 0.12s;
  }
  .chev.open {
    transform: none;
  }
  .reset {
    height: 24px;
    padding: 0 10px;
    border-radius: var(--radius-small);
    color: var(--accent-bright);
    font-size: 12px;
  }
  .reset:hover {
    background: var(--accent-wash);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 6px 18px;
    max-height: 168px;
    overflow-y: auto;
    padding: 2px 14px 12px;
  }
  .control {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .label {
    font-size: 12px;
    color: var(--text-dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .slider-row,
  .color-row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 24px;
  }
  input[type="range"] {
    flex: 1;
    min-width: 0;
    margin: 0;
    accent-color: var(--accent);
  }
  input[type="color"] {
    width: 34px;
    height: 22px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 5px;
    background: none;
    cursor: pointer;
  }
  input[type="color"]::-webkit-color-swatch-wrapper {
    padding: 2px;
  }
  input[type="color"]::-webkit-color-swatch {
    border: none;
    border-radius: 3px;
  }
  .value {
    flex: none;
    min-width: 42px;
    text-align: right;
    font-family: var(--font-code);
    font-size: 11.5px;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  .color-row .value {
    text-align: left;
    color: var(--text-dim);
  }
</style>
