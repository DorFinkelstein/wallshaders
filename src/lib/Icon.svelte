<script lang="ts" module>
  // 24x24 stroke icons.
  const PATHS: Record<string, string> = {
    library: "M4 5h7v6H4zM13 5h7v6h-7zM4 13h7v6H4zM13 13h7v6h-7z",
    code: "M9 7l-5 5 5 5M15 7l5 5-5 5",
    plus: "M12 5v14M5 12h14",
    import: "M12 4v11M7.5 10.5L12 15l4.5-4.5M5 19h14",
    export: "M12 15V4M7.5 8.5L12 4l4.5 4.5M5 19h14",
    settings: "M5 7h9M18 7h1M5 12h2M11 12h8M5 17h7M16 17h3M16 5v4M9 10v4M14 15v4",
    play: "M8 5.5v13l10.5-6.5z",
    pause: "M8 5v14M16 5v14",
    restart: "M5 12a7 7 0 1 0 2.2-5.1M5 4v4h4",
    close: "M6 6l12 12M18 6L6 18",
    edit: "M4 20h4L19 9l-4-4L4 16zM13.5 6.5l4 4",
    copy: "M9 9h10v10H9zM5 15V5h10",
    trash: "M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13",
    check: "M5 12.5l4.5 4.5L19 7.5",
    stop: "M7 7h10v10H7z",
    monitor: "M4 5h16v11H4zM9 20h6M12 16v4",
    search: "M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM16 16l4 4",
    alert: "M12 4l9 16H3zM12 10v4M12 17v.5",
    chevron: "M7 10l5 5 5-5",
    sliders: "M4 7h10M18 7h2M4 17h4M12 17h8M16 5v4M10 15v4",
    update: "M12 4v10M7.5 9.5L12 14l4.5-4.5M5 19h14",
  };
</script>

<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.7"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={PATHS[name] ?? ""} />
</svg>
