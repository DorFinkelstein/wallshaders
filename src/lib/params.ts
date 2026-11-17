// Shader settings: global constants marked with a comment get a control in the app.
//
//   float speed = 1.0;                    // @slider 0.1 3.0 Speed
//   const int layers = 3;                 // @slider 1 6
//   vec3 tint = vec3(0.85, 0.56, 0.45);   // @color Sky tint
//
// The file stays ordinary GLSL, so it works anywhere. Moving a control rewrites
// the number in the code. The preview turns these lines into uniforms so that
// dragging a slider does not recompile the shader.

export type ParamKind = "float" | "int" | "color";

export interface Param {
  name: string;
  kind: ParamKind;
  label: string;
  /** 0-based line in the script. */
  line: number;
  /** float/int: the number. color: [r, g, b] from 0 to 1. */
  value: number | [number, number, number];
  /** The value exactly as written in the script. */
  text: string;
  min: number;
  max: number;
  step: number;
}

const NUMBER = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/;
const DECLARATION =
  /^(\s*(?:const\s+)?(float|int|vec3)\s+([A-Za-z_]\w*)\s*=\s*)([^;]+?)(\s*;\s*\/\/\s*@(slider|color)\b(.*))$/;

function parseNumber(text: string): number | null {
  const t = text.trim();
  return NUMBER.test(t) ? Number(t) : null;
}

function parseVec3(text: string): [number, number, number] | null {
  const m = /^vec3\s*\(([^()]*)\)$/.exec(text.trim());
  if (!m) return null;
  const parts = m[1].split(",").map(parseNumber);
  if (parts.some((p) => p === null)) return null;
  const n = parts as number[];
  if (n.length === 1) return [n[0], n[0], n[0]];
  if (n.length === 3) return [n[0], n[1], n[2]];
  return null;
}

/** "starDensity" or "star_density" → "Star density". */
export function labelFromName(name: string): string {
  const words = name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/_+/g, " ")
    .trim()
    .toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Strip comments for brace counting, so `{` in a comment is ignored. */
function codeOnly(line: string, inBlock: { on: boolean }): string {
  let out = "";
  let i = 0;
  while (i < line.length) {
    if (inBlock.on) {
      const end = line.indexOf("*/", i);
      if (end < 0) return out;
      inBlock.on = false;
      i = end + 2;
    } else if (line.startsWith("//", i)) {
      return out;
    } else if (line.startsWith("/*", i)) {
      inBlock.on = true;
      i += 2;
    } else {
      out += line[i++];
    }
  }
  return out;
}

/** Find the marked constants. Only top-level declarations count. */
export function parseParams(source: string): Param[] {
  const params: Param[] = [];
  const seen = new Set<string>();
  const lines = source.split("\n");
  const block = { on: false };
  let depth = 0;
  lines.forEach((raw, index) => {
    const line = raw.replace(/\r$/, "");
    const atTop = depth === 0 && !block.on;
    const code = codeOnly(line, block);
    for (const c of code) {
      if (c === "{") depth++;
      else if (c === "}") depth = Math.max(0, depth - 1);
    }
    if (!atTop) return;
    const m = DECLARATION.exec(line);
    if (!m) return;
    const [, , type, name, valueText, , marker, rest] = m;
    if (seen.has(name)) return;
    const args = rest.trim().split(/\s+/).filter(Boolean);

    if (marker === "color") {
      if (type !== "vec3") return;
      const value = parseVec3(valueText);
      if (!value) return;
      seen.add(name);
      params.push({
        name,
        kind: "color",
        label: args.join(" ") || labelFromName(name),
        line: index,
        value,
        text: valueText,
        min: 0,
        max: 1,
        step: 0.001,
      });
      return;
    }

    if (type === "vec3") return;
    const value = parseNumber(valueText);
    const min = args.length > 0 ? parseNumber(args[0]) : null;
    const max = args.length > 1 ? parseNumber(args[1]) : null;
    if (value === null || min === null || max === null || !(max > min)) return;
    const label = args.slice(2).join(" ") || labelFromName(name);
    const kind: ParamKind = type === "int" ? "int" : "float";
    const step = kind === "int" ? 1 : niceStep((max - min) / 200);
    seen.add(name);
    params.push({ name, kind, label, line: index, value: kind === "int" ? Math.round(value) : value, text: valueText, min, max, step });
  });
  return params;
}

/** Round a step to 1, 2 or 5 times a power of ten. */
function niceStep(raw: number): number {
  const p = Math.pow(10, Math.floor(Math.log10(raw)));
  const f = raw / p;
  return (f < 1.5 ? 1 : f < 3.5 ? 2 : 5) * p;
}

function decimalsFor(step: number): number {
  return Math.min(5, Math.max(1, Math.ceil(-Math.log10(step) - 1e-9)));
}

/** GLSL text for a value. Floats always keep a decimal point. */
export function formatValue(param: Param, value: Param["value"]): string {
  if (param.kind === "color") {
    const [r, g, b] = value as [number, number, number];
    return `vec3(${r.toFixed(3)}, ${g.toFixed(3)}, ${b.toFixed(3)})`;
  }
  const n = value as number;
  if (param.kind === "int") return String(Math.round(n));
  return n.toFixed(decimalsFor(param.step));
}

/** Replace the value text on a constant's line. */
function setParamText(source: string, param: Param, text: string): string {
  const lines = source.split("\n");
  const line = lines[param.line];
  if (line === undefined) return source;
  const cr = line.endsWith("\r") ? "\r" : "";
  const m = DECLARATION.exec(line.replace(/\r$/, ""));
  if (!m || m[3] !== param.name) return source;
  lines[param.line] = m[1] + text + m[5] + cr;
  return lines.join("\n");
}

/** Rewrite one constant's value in the source. */
export function setParam(source: string, param: Param, value: Param["value"]): string {
  return setParamText(source, param, formatValue(param, value));
}

function sameValue(a: Param["value"], b: Param["value"]): boolean {
  if (Array.isArray(a) && Array.isArray(b)) return a.every((v, i) => Math.abs(v - b[i]) < 1e-6);
  return typeof a === "number" && typeof b === "number" && Math.abs(a - b) < 1e-9;
}

/** The settings whose value differs from the same setting in `from`, with the text they had there. */
function changedParams(source: string, from: string): { param: Param; text: string }[] {
  const old = new Map(parseParams(from).map((p) => [p.name, p]));
  const out: { param: Param; text: string }[] = [];
  for (const p of parseParams(source)) {
    const before = old.get(p.name);
    if (before && before.kind === p.kind && !sameValue(before.value, p.value)) out.push({ param: p, text: before.text });
  }
  return out;
}

/** Does any setting differ from the same setting in `from`? */
export function paramsDiffer(source: string, from: string): boolean {
  return changedParams(source, from).length > 0;
}

/** Put back the values a script had at another point (such as when it was last
 *  saved). Lines whose value already matches are left exactly as they are. */
export function resetParams(source: string, from: string): string {
  let out = source;
  for (const { param, text } of changedParams(source, from)) out = setParamText(out, param, text);
  return out;
}

/** The script with each marked constant turned into a uniform. Line numbers are kept. */
export function withUniforms(source: string, params: Param[]): string {
  if (!params.length) return source;
  const lines = source.split("\n");
  for (const p of params) {
    const type = p.kind === "color" ? "vec3" : p.kind;
    lines[p.line] = `uniform ${type} ${p.name};`;
  }
  return lines.join("\n");
}

export function toHex([r, g, b]: [number, number, number]): string {
  const c = (v: number) =>
    Math.round(Math.min(1, Math.max(0, v)) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

export function fromHex(hex: string): [number, number, number] {
  const n = parseInt(hex.replace("#", ""), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}
