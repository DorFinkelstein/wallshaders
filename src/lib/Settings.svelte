<script lang="ts">
  import type { Config } from "./api";

  interface Props {
    config: Config;
    autostart: boolean;
    version: string;
    checking: boolean;
    onchange: (config: Config) => void;
    onautostart: (enabled: boolean) => void;
    onrestore: () => void;
    oncheckupdates: () => void;
  }
  let { config, autostart, version, checking, onchange, onautostart, onrestore, oncheckupdates }: Props = $props();

  const FPS = [15, 24, 30, 60];
  const BATTERY: { value: Config["battery_mode"]; label: string }[] = [
    { value: "none", label: "No change" },
    { value: "half", label: "Half speed" },
    { value: "pause", label: "Pause" },
  ];

  function set<K extends keyof Config>(key: K, value: Config[K]) {
    onchange({ ...config, [key]: value });
  }
</script>

{#snippet toggle(label: string, hint: string, checked: boolean, onflip: (v: boolean) => void)}
  <label class="toggle">
    <span class="text">
      <span>{label}</span>
      <small>{hint}</small>
    </span>
    <input type="checkbox" {checked} onchange={(e) => onflip(e.currentTarget.checked)} />
    <span class="switch"></span>
  </label>
{/snippet}

<div class="panel">
  <h2>Settings</h2>

  <section>
    <h3>Performance</h3>
    <div class="field">
      <span>Frame rate</span>
      <div class="segments">
        {#each FPS as value}
          <button class:on={config.fps === value} onclick={() => set("fps", value)}>{value}</button>
        {/each}
      </div>
      <small>Lower is lighter. 30 looks smooth for slow shaders.</small>
    </div>
    <div class="field">
      <span>Detail <b>{Math.round(config.render_scale * 100)}%</b></span>
      <input
        type="range"
        min="25"
        max="100"
        step="5"
        value={Math.round(config.render_scale * 100)}
        onchange={(e) => set("render_scale", Number(e.currentTarget.value) / 100)}
      />
      <small>Draws the wallpaper at a smaller size and stretches it. The biggest saving on weak graphics cards.</small>
    </div>
  </section>

  <section>
    <h3>Pause automatically</h3>
    {@render toggle("Fullscreen apps and games", "Stops drawing while one is in front.", config.pause_fullscreen, (v) =>
      set("pause_fullscreen", v),
    )}
    {@render toggle("Screen is covered", "Stops drawing on a screen while a maximized window hides it.", config.pause_maximized, (v) =>
      set("pause_maximized", v),
    )}
    <div class="field">
      <span>On battery</span>
      <div class="segments">
        {#each BATTERY as option}
          <button class:on={config.battery_mode === option.value} onclick={() => set("battery_mode", option.value)}>
            {option.label}
          </button>
        {/each}
      </div>
    </div>
  </section>

  <section>
    <h3>General</h3>
    {@render toggle("Start with Windows", "Shows your wallpaper after you sign in.", autostart, onautostart)}
    {@render toggle("Follow the mouse", "Lets shaders that use iMouse react to the cursor.", config.mouse_input, (v) =>
      set("mouse_input", v),
    )}
    <button class="btn restore" onclick={onrestore}>Restore the starter shaders</button>
  </section>

  <section>
    <h3>Updates</h3>
    {@render toggle("Check for updates", "Looks for a new version when Wallshaders opens.", config.check_updates, (v) =>
      set("check_updates", v),
    )}
    <div class="version-row">
      <span class="version">Version {version}</span>
      <button class="btn" onclick={oncheckupdates} disabled={checking}>{checking ? "Checking…" : "Check now"}</button>
    </div>
  </section>
</div>

<style>
  .panel {
    height: 100%;
    overflow-y: auto;
    padding: 16px 16px 20px;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 15px;
    font-weight: 600;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px 0 18px;
    border-bottom: 1px solid var(--line);
  }
  section:last-child {
    border-bottom: none;
  }
  h3 {
    margin: 0;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
  }
  small {
    display: block;
    color: var(--text-faint);
    font-size: 12px;
    line-height: 1.4;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .field b {
    color: var(--accent-bright);
    font-weight: 600;
    margin-left: 4px;
  }
  .segments {
    display: flex;
    padding: 3px;
    gap: 3px;
    border-radius: 9px;
    background: var(--bg-0);
  }
  .segments button {
    flex: 1;
    height: 26px;
    border-radius: 7px;
    color: var(--text-dim);
    font-size: 12.5px;
  }
  .segments button:hover {
    color: var(--text);
  }
  .segments button.on {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
    margin: 0;
  }
  .toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    cursor: pointer;
  }
  .toggle .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .toggle input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .switch {
    flex: none;
    width: 36px;
    height: 20px;
    border-radius: 20px;
    background: var(--bg-0);
    border: 1px solid var(--line);
    position: relative;
    transition: background 0.15s, border-color 0.15s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--text-dim);
    transition: transform 0.15s, background 0.15s;
  }
  .toggle input:checked + .switch {
    background: var(--accent);
    border-color: var(--accent);
  }
  .toggle input:checked + .switch::after {
    transform: translateX(16px);
    background: var(--on-accent);
  }
  .toggle input:focus-visible + .switch {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .restore {
    align-self: flex-start;
  }
  .version-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .version {
    color: var(--text-dim);
    font-variant-numeric: tabular-nums;
  }
</style>
