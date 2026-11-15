<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { VERTEX, parseErrors, wrapFragment, type ShaderError } from "./glsl";

  interface Props {
    name: string;
    source: string;
    /** Frame-rate cap, the same one the wallpaper uses. */
    fps: number;
    onerrors: (errors: ShaderError[]) => void;
  }
  let { name, source, fps, onerrors }: Props = $props();

  let canvas: HTMLCanvasElement;
  let stage: HTMLDivElement;
  let gl: WebGL2RenderingContext | null = null;
  let program: WebGLProgram | null = null;
  let uniforms: Record<string, WebGLUniformLocation | null> = {};

  let playing = $state(true);
  let measuredFps = $state(0);
  let resolution = $state("");
  let unsupported = $state(false);

  let time = 0;
  let frame = 0;
  let lastDraw = 0;
  let redraw = true;
  let mouse = [0, 0];
  let framesThisSecond = 0;
  let secondStart = 0;

  // Shape the preview like the screen the wallpaper will cover.
  const screenOk = window.screen.width > 0 && window.screen.height > 0;
  const aspect = screenOk ? `${window.screen.width} / ${window.screen.height}` : "16 / 9";

  function stageShader(kind: number, text: string): WebGLShader | string {
    const sh = gl!.createShader(kind)!;
    gl!.shaderSource(sh, text);
    gl!.compileShader(sh);
    if (gl!.getShaderParameter(sh, gl!.COMPILE_STATUS)) return sh;
    const log = gl!.getShaderInfoLog(sh) ?? "Compile failed";
    gl!.deleteShader(sh);
    return log;
  }

  /** Compile a script. If it fails, the last working picture stays on screen. */
  function compile(text: string) {
    if (!gl) return;
    const vs = stageShader(gl.VERTEX_SHADER, VERTEX);
    const fs = stageShader(gl.FRAGMENT_SHADER, wrapFragment(text));
    if (typeof vs === "string" || typeof fs === "string") {
      if (typeof vs !== "string") gl.deleteShader(vs);
      onerrors(parseErrors(typeof fs === "string" ? fs : (vs as string)));
      return;
    }
    const next = gl.createProgram()!;
    gl.attachShader(next, vs);
    gl.attachShader(next, fs);
    gl.linkProgram(next);
    gl.deleteShader(vs);
    gl.deleteShader(fs);
    if (!gl.getProgramParameter(next, gl.LINK_STATUS)) {
      onerrors(parseErrors(gl.getProgramInfoLog(next) ?? "Link failed", true));
      gl.deleteProgram(next);
      return;
    }
    if (program) gl.deleteProgram(program);
    program = next;
    uniforms = {};
    for (const u of ["iResolution", "iTime", "iTimeDelta", "iFrameRate", "iFrame", "iMouse", "iDate"]) {
      uniforms[u] = gl.getUniformLocation(next, u);
    }
    redraw = true;
    onerrors([]);
  }

  function draw(dt: number) {
    if (!gl || !program) return;
    const d = new Date();
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.useProgram(program);
    gl.uniform3f(uniforms.iResolution, canvas.width, canvas.height, 1);
    gl.uniform1f(uniforms.iTime, time);
    gl.uniform1f(uniforms.iTimeDelta, dt);
    gl.uniform1f(uniforms.iFrameRate, fps);
    gl.uniform1i(uniforms.iFrame, frame);
    gl.uniform4f(uniforms.iMouse, mouse[0], mouse[1], 0, 0);
    gl.uniform4f(
      uniforms.iDate,
      d.getFullYear(),
      d.getMonth(),
      d.getDate(),
      d.getHours() * 3600 + d.getMinutes() * 60 + d.getSeconds() + d.getMilliseconds() / 1000,
    );
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    frame++;
  }

  function fit() {
    const scale = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.round(stage.clientWidth * scale));
    const h = Math.max(1, Math.round(stage.clientHeight * scale));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
      resolution = `${w} × ${h}`;
      redraw = true;
    }
  }

  function restart() {
    time = 0;
    frame = 0;
    redraw = true;
  }

  function onpointermove(e: PointerEvent) {
    const r = canvas.getBoundingClientRect();
    mouse = [((e.clientX - r.left) / r.width) * canvas.width, (1 - (e.clientY - r.top) / r.height) * canvas.height];
    redraw = true;
  }

  onMount(() => {
    gl = canvas.getContext("webgl2", { antialias: false, alpha: false, powerPreference: "low-power" });
    if (!gl) {
      unsupported = true;
      return;
    }
    gl.bindVertexArray(gl.createVertexArray());
    const observer = new ResizeObserver(fit);
    observer.observe(stage);
    fit();

    let raf = 0;
    const tick = (now: number) => {
      raf = requestAnimationFrame(tick);
      if (now - secondStart >= 1000) {
        measuredFps = Math.round((framesThisSecond * 1000) / (now - secondStart));
        framesThisSecond = 0;
        secondStart = now;
      }
      if (!playing && !redraw) return;
      if (playing && !redraw && now - lastDraw < 1000 / fps - 2) return;
      const dt = playing ? Math.min((now - lastDraw) / 1000, 0.1) : 0;
      lastDraw = now;
      time += dt;
      redraw = false;
      draw(dt);
      framesThisSecond++;
    };
    raf = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
    };
  });

  // Recompile shortly after the script stops changing.
  $effect(() => {
    const text = source;
    const timer = setTimeout(() => compile(text), 180);
    return () => clearTimeout(timer);
  });
</script>

<div class="preview">
  <header>
    <div class="title">
      <span class="label">Preview</span>
      <span class="name">{name}</span>
    </div>
    <div class="meta">
      <span class="stat" title="Frames per second in this preview">{playing ? measuredFps : 0} fps</span>
      <span class="stat">{resolution}</span>
      <button class="tool" onclick={restart} title="Restart from the beginning"><Icon name="restart" size={16} /></button>
      <button class="tool" onclick={() => (playing = !playing)} title={playing ? "Pause preview" : "Play preview"}>
        <Icon name={playing ? "pause" : "play"} size={16} />
      </button>
    </div>
  </header>
  <div class="frame">
    <div class="stage" bind:this={stage} style:aspect-ratio={aspect} style:--aspect={aspect}>
      <canvas bind:this={canvas} {onpointermove}></canvas>
      {#if unsupported}
        <p class="unsupported">This computer's graphics driver does not support WebGL2, so the preview is unavailable.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    height: 40px;
    padding: 0 10px 0 16px;
    flex: none;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
  }
  .label {
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .name {
    color: var(--text);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }
  .stat {
    font-family: var(--font-code);
    font-size: 11.5px;
    color: var(--text-dim);
    margin-right: 10px;
    font-variant-numeric: tabular-nums;
  }
  .tool {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-small);
    color: var(--text-dim);
  }
  .tool:hover {
    background: var(--bg-3);
    color: var(--text);
  }
  .frame {
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
    padding: 6px 18px 14px;
    container-type: size;
  }
  .stage {
    position: relative;
    width: min(100cqw, calc(100cqh * var(--aspect)));
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--bg-0);
    box-shadow: var(--shadow);
  }
  canvas {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
  }
  .unsupported {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    padding: 24px;
    text-align: center;
    color: var(--text-dim);
  }
</style>
