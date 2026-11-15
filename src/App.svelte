<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import * as api from "./lib/api";
  import type { Config } from "./lib/api";
  import { NEW_SHADER_TEMPLATE, parseErrors, type ShaderError } from "./lib/glsl";
  import Editor from "./lib/Editor.svelte";
  import Icon from "./lib/Icon.svelte";
  import Library from "./lib/Library.svelte";
  import Preview from "./lib/Preview.svelte";
  import Settings from "./lib/Settings.svelte";

  interface Tab {
    id: number;
    name: string;
    source: string;
    /** Text as it is on disk; differs from `source` while there are unsaved edits. */
    saved: string;
  }

  interface Question {
    title: string;
    body: string;
    options: { label: string; value: string; primary?: boolean }[];
    resolve: (value: string) => void;
  }

  let shaders = $state<string[]>([]);
  let tabs = $state<Tab[]>([]);
  let activeId = $state<number | null>(null);
  let editorOpen = $state(true);
  let drawer = $state<"library" | "settings" | null>(null);
  let config = $state<Config | null>(null);
  let autostart = $state(false);
  let status = $state<api.Status>({ running: false, active: null });
  let previewErrors = $state<ShaderError[]>([]);
  let applyErrors = $state<ShaderError[]>([]);
  let applying = $state(false);
  let toast = $state<{ text: string; bad: boolean } | null>(null);
  let question = $state<Question | null>(null);
  let editorWidth = $state(46); // percent of the space shared with the preview
  let workspace: HTMLDivElement;
  let editor = $state<ReturnType<typeof Editor>>();

  let nextId = 1;
  let toastTimer = 0;

  const active = $derived(tabs.find((t) => t.id === activeId) ?? null);
  const errors = $derived(previewErrors.length ? previewErrors : applyErrors);
  const showEditor = $derived(editorOpen && active !== null);

  function notify(text: string, bad = false) {
    toast = { text, bad };
    clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => (toast = null), bad ? 6000 : 2600);
  }

  /** Run a backend call; show a message instead of throwing if it fails. */
  async function attempt<T>(work: () => Promise<T>): Promise<T | undefined> {
    try {
      return await work();
    } catch (e) {
      notify(api.errorText(e), true);
      return undefined;
    }
  }

  function ask(title: string, body: string, options: Question["options"]): Promise<string> {
    return new Promise((resolve) => {
      question = { title, body, options, resolve };
    });
  }

  function answer(value: string) {
    question?.resolve(value);
    question = null;
  }

  async function refresh() {
    shaders = (await attempt(api.listShaders)) ?? shaders;
    status = (await attempt(api.wallpaperStatus)) ?? status;
  }

  async function openShader(name: string) {
    const existing = tabs.find((t) => t.name === name);
    if (existing) {
      activeId = existing.id;
    } else {
      const source = await attempt(() => api.readShader(name));
      if (source === undefined) return;
      const tab: Tab = { id: nextId++, name, source, saved: source };
      tabs.push(tab);
      activeId = tab.id;
    }
    applyErrors = [];
    drawer = null;
  }

  async function save(tab: Tab): Promise<boolean> {
    const text = tab.source;
    const ok = (await attempt(() => api.saveShader(tab.name, text).then(() => true))) ?? false;
    if (ok) tab.saved = text;
    return ok;
  }

  async function closeTab(tab: Tab) {
    if (tab.source !== tab.saved) {
      const choice = await ask(`Save changes to “${tab.name}”?`, "Your edits will be lost if you don't save them.", [
        { label: "Cancel", value: "cancel" },
        { label: "Don't save", value: "discard" },
        { label: "Save", value: "save", primary: true },
      ]);
      if (choice === "cancel" || (choice === "save" && !(await save(tab)))) return;
    }
    const index = tabs.findIndex((t) => t.id === tab.id);
    if (index < 0) return;
    tabs.splice(index, 1);
    editor?.forget(tab.id);
    if (activeId === tab.id) activeId = (tabs[index] ?? tabs[index - 1])?.id ?? null;
    applyErrors = [];
  }

  function uniqueName(base: string): string {
    const taken = new Set(shaders.map((s) => s.toLowerCase()));
    if (!taken.has(base.toLowerCase())) return base;
    for (let i = 2; ; i++) {
      if (!taken.has(`${base} ${i}`.toLowerCase())) return `${base} ${i}`;
    }
  }

  async function createShader(base: string, source: string) {
    const name = uniqueName(base);
    if ((await attempt(() => api.saveShader(name, source).then(() => true))) !== true) return;
    await refresh();
    editorOpen = true;
    await openShader(name);
  }

  const newShader = () => createShader("Untitled", NEW_SHADER_TEMPLATE);

  async function duplicateShader(name: string) {
    const open = tabs.find((t) => t.name === name);
    const source = open?.source ?? (await attempt(() => api.readShader(name)));
    if (source !== undefined) await createShader(`${name} copy`, source);
  }

  async function importShaders() {
    const names = await attempt(api.importShaders);
    if (!names?.length) return;
    await refresh();
    await openShader(names[names.length - 1]);
    notify(names.length === 1 ? `Imported “${names[0]}”` : `Imported ${names.length} shaders`);
  }

  async function renameShader(name: string, newName: string) {
    const result = await attempt(() => api.renameShader(name, newName));
    if (result === undefined) return;
    const tab = tabs.find((t) => t.name === name);
    if (tab) tab.name = result;
    await refresh();
  }

  async function exportShader(name: string) {
    const tab = tabs.find((t) => t.name === name);
    if (tab && tab.source !== tab.saved && !(await save(tab))) return;
    if (await attempt(() => api.exportShader(name))) notify(`Exported “${name}”`);
  }

  async function deleteShader(name: string) {
    const onDesktop = status.active === name;
    const choice = await ask(
      `Delete “${name}”?`,
      onDesktop ? "It is your current wallpaper, so the wallpaper will stop too. This cannot be undone." : "This cannot be undone.",
      [
        { label: "Cancel", value: "cancel" },
        { label: "Delete", value: "delete", primary: true },
      ],
    );
    if (choice !== "delete") return;
    if ((await attempt(() => api.deleteShader(name).then(() => true))) !== true) return;
    const index = tabs.findIndex((t) => t.name === name);
    if (index >= 0) {
      const [tab] = tabs.splice(index, 1);
      editor?.forget(tab.id);
      if (activeId === tab.id) activeId = (tabs[index] ?? tabs[index - 1])?.id ?? null;
    }
    await refresh();
  }

  async function applyWallpaper() {
    const tab = active;
    if (!tab || applying) return;
    if (previewErrors.length) {
      notify("Fix the errors in this shader before using it as wallpaper.", true);
      return;
    }
    applying = true;
    applyErrors = [];
    try {
      if (tab.source !== tab.saved && !(await save(tab))) return;
      await api.applyWallpaper(tab.name);
      notify(`“${tab.name}” is now your wallpaper`);
    } catch (e) {
      // The wallpaper renderer rejected the script; its log already uses script line numbers.
      applyErrors = parseErrors(api.errorText(e), true);
      notify("The wallpaper renderer could not compile this shader.", true);
    } finally {
      applying = false;
      status = (await attempt(api.wallpaperStatus)) ?? status;
    }
  }

  async function stopWallpaper() {
    await attempt(api.stopWallpaper);
    status = { running: false, active: null };
    notify("Wallpaper stopped");
  }

  async function changeConfig(next: Config) {
    config = next;
    await attempt(() => api.setConfig(next));
  }

  async function changeAutostart(enabled: boolean) {
    autostart = enabled;
    if ((await attempt(() => api.setAutostart(enabled).then(() => true))) !== true) autostart = !enabled;
  }

  async function restoreBuiltins() {
    if ((await attempt(() => api.restoreBuiltins().then(() => true))) !== true) return;
    await refresh();
    notify("Starter shaders restored");
  }

  function toggleDrawer(which: "library" | "settings") {
    drawer = drawer === which ? null : which;
  }

  function startResize(e: PointerEvent) {
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      const box = workspace.getBoundingClientRect();
      editorWidth = Math.min(75, Math.max(25, ((ev.clientX - box.left) / box.width) * 100));
    };
    const stop = () => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", stop);
    };
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", stop);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (question) answer("cancel");
      else drawer = null;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      if (active) save(active);
    }
  }

  onMount(() => {
    (async () => {
      config = (await attempt(api.getConfig)) ?? null;
      autostart = (await attempt(api.getAutostart)) ?? false;
      await refresh();
      const first = status.active ?? config?.active_shader ?? shaders[0];
      if (first && shaders.includes(first)) await openShader(first);
      else if (shaders.length) await openShader(shaders[0]);
      else drawer = "library";
    })();

    // The wallpaper can be stopped from its tray icon, so keep the status fresh.
    const poll = setInterval(async () => {
      if (!document.hidden) status = await api.wallpaperStatus().catch(() => status);
    }, 4000);

    let unlisten = () => {};
    if (api.inApp) {
      const win = getCurrentWindow();
      win
        .onCloseRequested(async (event) => {
          const dirty = tabs.filter((t) => t.source !== t.saved);
          if (!dirty.length) return;
          event.preventDefault();
          const choice = await ask("Save your changes before closing?", `${dirty.length === 1 ? `“${dirty[0].name}” has` : `${dirty.length} shaders have`} unsaved edits.`, [
            { label: "Cancel", value: "cancel" },
            { label: "Don't save", value: "discard" },
            { label: "Save all", value: "save", primary: true },
          ]);
          if (choice === "cancel") return;
          if (choice === "save") {
            for (const tab of dirty) if (!(await save(tab))) return;
          }
          await win.destroy();
        })
        .then((fn) => (unlisten = fn));
    }
    return () => {
      clearInterval(poll);
      unlisten();
    };
  });
</script>

<svelte:window {onkeydown} />

<div class="app">
  <nav class="toolbar" aria-label="Main">
    <div class="logo" title="Wallshaders">
      <svg viewBox="0 0 32 32" width="30" height="30" aria-hidden="true">
        <rect width="32" height="32" rx="9" fill="var(--accent)" />
        <circle cx="20.5" cy="11.5" r="4.5" fill="#fff0de" />
        <path d="M0 18c6-3 11-2 17 1s10 3 15 1v3a9 9 0 0 1-9 9H9a9 9 0 0 1-9-9z" fill="#8f5546" />
        <path d="M0 24c5-3 10-3 16-1s11 2 16 0v0a9 9 0 0 1-9 9H9a9 9 0 0 1-9-9z" fill="#33302f" />
      </svg>
    </div>
    <button class:on={drawer === "library"} onclick={() => toggleDrawer("library")} title="Library">
      <Icon name="library" size={21} />
    </button>
    <button
      class:on={showEditor}
      onclick={() => (editorOpen = !editorOpen)}
      disabled={!active}
      title={editorOpen ? "Hide the script editor" : "Show the script editor"}
    >
      <Icon name="code" size={21} />
    </button>
    <button onclick={newShader} title="New shader"><Icon name="plus" size={21} /></button>
    <button onclick={importShaders} title="Import shaders from files"><Icon name="import" size={21} /></button>
    <div class="spacer"></div>
    <button class:on={drawer === "settings"} onclick={() => toggleDrawer("settings")} title="Settings">
      <Icon name="settings" size={21} />
    </button>
  </nav>

  <div class="workspace" bind:this={workspace}>
    {#if showEditor && active}
      <section class="editor-pane" style:width="{editorWidth}%">
        <div class="tabs" role="tablist">
          {#each tabs as tab (tab.id)}
            <div class="tab" class:on={tab.id === activeId}>
              <button class="tab-name" role="tab" aria-selected={tab.id === activeId} onclick={() => ((activeId = tab.id), (applyErrors = []))}>
                {tab.name}
              </button>
              <button class="tab-close" class:dirty={tab.source !== tab.saved} onclick={() => closeTab(tab)} title="Close">
                <span class="dot"></span>
                <Icon name="close" size={13} />
              </button>
            </div>
          {/each}
        </div>
        <div class="editor-body">
          <Editor
            bind:this={editor}
            docId={active.id}
            source={active.source}
            {errors}
            onchange={(text) => {
              if (active) active.source = text;
              applyErrors = [];
            }}
            onsave={() => active && save(active)}
          />
        </div>
      </section>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="splitter" onpointerdown={startResize}></div>
    {/if}

    <section class="preview-pane">
      {#if active}
        <Preview name={active.name} source={active.source} fps={config?.fps ?? 30} onerrors={(list) => (previewErrors = list)} />

        {#if errors.length}
          <div class="errors" role="alert">
            <div class="errors-title"><Icon name="alert" size={15} />{errors.length === 1 ? "1 problem" : `${errors.length} problems`} in this shader</div>
            <ul>
              {#each errors.slice(0, 6) as error}
                <li><span class="line">Line {error.line}</span>{error.message}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <footer>
          <div class="status">
            {#if status.running && status.active}
              <span class="live"></span>
              <span class="status-text">On desktop: <b>{status.active}</b></span>
              <button class="btn" onclick={stopWallpaper}><Icon name="stop" size={14} />Stop</button>
            {:else}
              <span class="status-text dim">No shader wallpaper is running</span>
            {/if}
          </div>
          <button class="btn primary" onclick={applyWallpaper} disabled={applying}>
            <Icon name="monitor" size={16} />
            {applying ? "Applying…" : status.running && status.active === active.name ? "Update wallpaper" : "Set as wallpaper"}
          </button>
        </footer>
      {:else}
        <div class="welcome">
          <h1>Nothing open yet</h1>
          <p>Pick a shader from the library, or start a new one.</p>
          <div class="welcome-actions">
            <button class="btn primary" onclick={() => (drawer = "library")}><Icon name="library" size={16} />Open the library</button>
            <button class="btn" onclick={newShader}><Icon name="plus" size={16} />New shader</button>
          </div>
        </div>
      {/if}
    </section>

    {#if drawer}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="scrim" onclick={() => (drawer = null)}></div>
      <aside class="drawer">
        {#if drawer === "library"}
          <Library
            {shaders}
            current={active?.name ?? null}
            wallpaper={status.running ? status.active : null}
            onopen={openShader}
            onnew={newShader}
            onimport={importShaders}
            onrename={renameShader}
            onduplicate={duplicateShader}
            onexport={exportShader}
            ondelete={deleteShader}
          />
        {:else if config}
          <Settings {config} {autostart} onchange={changeConfig} onautostart={changeAutostart} onrestore={restoreBuiltins} />
        {/if}
      </aside>
    {/if}
  </div>

  {#if toast}
    <div class="toast" class:bad={toast.bad} role="status">{toast.text}</div>
  {/if}

  {#if question}
    <div class="modal-scrim">
      <div class="modal" role="dialog" aria-modal="true" aria-label={question.title}>
        <h2>{question.title}</h2>
        <p>{question.body}</p>
        <div class="modal-actions">
          {#each question.options as option}
            <button class="btn" class:primary={option.primary} onclick={() => answer(option.value)}>{option.label}</button>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .app {
    display: flex;
    height: 100%;
  }

  /* ---- toolbar ---- */
  .toolbar {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 56px;
    flex: none;
    padding: 12px 0;
    background: var(--bg-0);
  }
  .logo {
    display: grid;
    place-items: center;
    margin-bottom: 10px;
  }
  .toolbar button {
    position: relative;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius);
    color: var(--text-dim);
    transition: background 0.12s, color 0.12s;
  }
  .toolbar button:hover:not(:disabled) {
    background: var(--bg-2);
    color: var(--text);
  }
  .toolbar button.on {
    background: var(--accent-wash);
    color: var(--accent-bright);
  }
  .toolbar button.on::before {
    content: "";
    position: absolute;
    left: -8px;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .toolbar button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .spacer {
    flex: 1;
  }

  /* ---- workspace ---- */
  .workspace {
    position: relative;
    display: flex;
    flex: 1;
    min-width: 0;
  }
  .editor-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: none;
    background: var(--bg-2);
  }
  .tabs {
    display: flex;
    flex: none;
    height: 40px;
    background: var(--bg-1);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    display: flex;
    align-items: center;
    flex: none;
    max-width: 200px;
    padding-right: 6px;
    color: var(--text-dim);
    border-top: 2px solid transparent;
  }
  .tab.on {
    background: var(--bg-2);
    color: var(--text);
    border-top-color: var(--accent);
    border-radius: 8px 8px 0 0;
  }
  .tab-name {
    padding: 0 6px 0 14px;
    height: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tab-close {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: var(--radius-small);
    color: var(--text-faint);
  }
  .tab-close .dot {
    display: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
  }
  .tab-close.dirty:not(:hover) .dot {
    display: block;
  }
  .tab-close.dirty:not(:hover) :global(svg) {
    display: none;
  }
  .tab-close:hover {
    background: var(--bg-3);
    color: var(--text);
  }
  .editor-body {
    flex: 1;
    min-height: 0;
  }
  .splitter {
    width: 5px;
    margin: 0 -2px;
    flex: none;
    cursor: col-resize;
    z-index: 2;
    touch-action: none;
  }
  .splitter:hover {
    background: var(--accent);
    opacity: 0.6;
  }
  .preview-pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    background: var(--bg-1);
  }

  /* ---- errors and footer ---- */
  .errors {
    flex: none;
    margin: 0 18px 12px;
    padding: 10px 14px;
    border-radius: var(--radius);
    background: rgba(229, 138, 120, 0.1);
    border: 1px solid rgba(229, 138, 120, 0.3);
    max-height: 150px;
    overflow-y: auto;
    user-select: text;
    -webkit-user-select: text;
  }
  .errors-title {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--bad);
    font-weight: 600;
  }
  .errors ul {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    font-family: var(--font-code);
    font-size: 12px;
    color: var(--text);
  }
  .errors li {
    padding: 2px 0;
    overflow-wrap: anywhere;
  }
  .errors .line {
    color: var(--bad);
    margin-right: 10px;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex: none;
    padding: 12px 18px;
    border-top: 1px solid var(--line);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .status-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status-text b {
    font-weight: 600;
  }
  .dim {
    color: var(--text-faint);
  }
  .live {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--good);
    box-shadow: 0 0 0 3px rgba(163, 189, 156, 0.18);
  }
  footer .btn.primary {
    height: 36px;
    padding: 0 18px;
  }

  /* ---- empty state ---- */
  .welcome {
    margin: auto;
    text-align: center;
    padding: 24px;
  }
  .welcome h1 {
    margin: 0 0 6px;
    font-size: 20px;
    font-weight: 600;
  }
  .welcome p {
    margin: 0 0 18px;
    color: var(--text-dim);
  }
  .welcome-actions {
    display: flex;
    gap: 10px;
    justify-content: center;
  }

  /* ---- drawer ---- */
  .scrim {
    position: absolute;
    inset: 0;
    z-index: 5;
    background: rgba(20, 18, 17, 0.35);
  }
  .drawer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 300px;
    z-index: 6;
    background: var(--bg-1);
    border-right: 1px solid var(--line);
    box-shadow: var(--shadow);
  }

  /* ---- toast and dialog ---- */
  .toast {
    position: fixed;
    left: 50%;
    bottom: 70px;
    transform: translateX(-50%);
    z-index: 20;
    max-width: min(520px, 80vw);
    padding: 9px 16px;
    border-radius: 20px;
    background: var(--bg-0);
    border: 1px solid var(--line);
    box-shadow: var(--shadow);
  }
  .toast.bad {
    border-color: rgba(229, 138, 120, 0.5);
    color: #f3c0b5;
  }
  .modal-scrim {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: grid;
    place-items: center;
    background: rgba(20, 18, 17, 0.55);
  }
  .modal {
    width: min(400px, 86vw);
    padding: 22px;
    border-radius: 14px;
    background: var(--bg-2);
    border: 1px solid var(--line);
    box-shadow: var(--shadow);
  }
  .modal h2 {
    margin: 0 0 6px;
    font-size: 16px;
    font-weight: 600;
  }
  .modal p {
    margin: 0 0 20px;
    color: var(--text-dim);
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
