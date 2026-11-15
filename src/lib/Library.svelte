<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    shaders: string[];
    /** Shader shown in the editor and preview. */
    current: string | null;
    /** Shader currently on the desktop. */
    wallpaper: string | null;
    onopen: (name: string) => void;
    onnew: () => void;
    onimport: () => void;
    onrename: (name: string, newName: string) => void;
    onduplicate: (name: string) => void;
    onexport: (name: string) => void;
    ondelete: (name: string) => void;
  }
  let { shaders, current, wallpaper, onopen, onnew, onimport, onrename, onduplicate, onexport, ondelete }: Props =
    $props();

  let query = $state("");
  let renaming = $state<string | null>(null);
  let draft = $state("");

  const visible = $derived(shaders.filter((s) => s.toLowerCase().includes(query.trim().toLowerCase())));

  function startRename(name: string) {
    renaming = name;
    draft = name;
  }

  function finishRename() {
    const from = renaming;
    renaming = null;
    if (from && draft.trim() && draft.trim() !== from) onrename(from, draft.trim());
  }

  function focusAndSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="panel">
  <h2>Library</h2>
  <div class="actions">
    <button class="btn" onclick={onnew}><Icon name="plus" size={15} />New</button>
    <button class="btn" onclick={onimport}><Icon name="import" size={15} />Import</button>
  </div>
  <label class="search">
    <Icon name="search" size={14} />
    <input placeholder="Search shaders" bind:value={query} spellcheck="false" />
  </label>

  <ul>
    {#each visible as name (name)}
      <li class:current={name === current}>
        {#if renaming === name}
          <input
            class="rename"
            bind:value={draft}
            use:focusAndSelect
            onblur={finishRename}
            onkeydown={(e) => {
              if (e.key === "Enter") finishRename();
              if (e.key === "Escape") renaming = null;
            }}
            maxlength="60"
            spellcheck="false"
          />
        {:else}
          <button class="open" onclick={() => onopen(name)} title="Open {name}">
            <span class="name">{name}</span>
            {#if name === wallpaper}
              <span class="badge" title="This shader is your wallpaper"><Icon name="monitor" size={12} />On desktop</span>
            {/if}
          </button>
          <div class="row-tools">
            <button onclick={() => startRename(name)} title="Rename"><Icon name="edit" size={14} /></button>
            <button onclick={() => onduplicate(name)} title="Duplicate"><Icon name="copy" size={14} /></button>
            <button onclick={() => onexport(name)} title="Export to a file"><Icon name="export" size={14} /></button>
            <button class="danger" onclick={() => ondelete(name)} title="Delete"><Icon name="trash" size={14} /></button>
          </div>
        {/if}
      </li>
    {:else}
      <li class="empty">{shaders.length ? "No shader matches that search." : "No shaders yet. Create or import one."}</li>
    {/each}
  </ul>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 16px 12px 12px;
    gap: 12px;
  }
  h2 {
    margin: 0 4px;
    font-size: 15px;
    font-weight: 600;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--bg-0);
    color: var(--text-faint);
  }
  .search input {
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--text-faint);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  li {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 8px;
    min-height: 36px;
  }
  li:hover,
  li:focus-within {
    background: var(--bg-3);
  }
  li.current {
    background: var(--accent-wash);
  }
  li.current .name {
    color: var(--accent-bright);
  }
  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    text-align: left;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
    padding: 1px 7px 1px 5px;
    border-radius: 20px;
    background: var(--accent);
    color: var(--on-accent);
    font-size: 10.5px;
    font-weight: 600;
  }
  .row-tools {
    display: none;
    flex: none;
    padding-right: 6px;
  }
  li:hover .row-tools,
  li:focus-within .row-tools {
    display: flex;
  }
  li:hover .badge,
  li:focus-within .badge {
    display: none;
  }
  .row-tools button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-small);
    color: var(--text-dim);
  }
  .row-tools button:hover {
    background: var(--line);
    color: var(--text);
  }
  .row-tools button.danger:hover {
    color: var(--bad);
  }
  .rename {
    flex: 1;
    min-width: 0;
    margin: 3px;
    height: 30px;
    padding: 0 8px;
    border-radius: var(--radius-small);
    border: 1px solid var(--accent);
    background: var(--bg-0);
    outline: none;
    user-select: text;
    -webkit-user-select: text;
  }
  .empty {
    padding: 14px 10px;
    color: var(--text-faint);
  }
  .empty:hover {
    background: none;
  }
</style>
