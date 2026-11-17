// Run with: npm run test:ui
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseParams, setParam, resetParams, paramsDiffer, withUniforms, labelFromName, toHex, fromHex } from "./params.ts";

const SCRIPT = [
  "// demo",
  "float speed = 1.0; // @slider 0.1 3.0 Wind speed",
  "const int layers = 3;   // @slider 1 6",
  "vec3 tint = vec3(0.85, 0.56, 0.45); // @color",
  "float plain = 2.0; // not a control",
  "float bad = 1.0; // @slider 5 1",
  "void mainImage(out vec4 c, in vec2 p) {",
  "    float inner = 1.0; // @slider 0 2",
  "    c = vec4(tint * speed, 1.0);",
  "}",
  "/* float hidden = 1.0; // @slider 0 2 */",
].join("\n");

test("finds top-level marked constants only", () => {
  const p = parseParams(SCRIPT);
  assert.deepEqual(
    p.map((x) => [x.name, x.kind, x.label, x.line]),
    [
      ["speed", "float", "Wind speed", 1],
      ["layers", "int", "Layers", 2],
      ["tint", "color", "Tint", 3],
    ],
  );
  assert.equal(p[0].value, 1);
  assert.equal(p[0].min, 0.1);
  assert.equal(p[0].max, 3);
  assert.equal(p[1].step, 1);
  assert.deepEqual(p[2].value, [0.85, 0.56, 0.45]);
});

test("rewrites a value and keeps the rest of the line", () => {
  const [speed, layers, tint] = parseParams(SCRIPT);
  let s = setParam(SCRIPT, speed, 2.25);
  assert.equal(s.split("\n")[1], "float speed = 2.25; // @slider 0.1 3.0 Wind speed");
  s = setParam(s, layers, 4.6);
  assert.equal(s.split("\n")[2], "const int layers = 5;   // @slider 1 6");
  s = setParam(s, tint, [1, 0.5, 0]);
  assert.equal(s.split("\n")[3], "vec3 tint = vec3(1.000, 0.500, 0.000); // @color");
  assert.equal(s.split("\n").length, SCRIPT.split("\n").length);
  // Floats always keep a decimal point, so the GLSL stays valid.
  assert.match(setParam(SCRIPT, speed, 2).split("\n")[1], /= 2\.0+;/);
});

test("reset puts back the saved values and leaves the rest alone", () => {
  const [speed] = parseParams(SCRIPT);
  assert.equal(paramsDiffer(SCRIPT, SCRIPT), false);
  assert.equal(resetParams(SCRIPT, SCRIPT), SCRIPT);
  const changed = setParam(SCRIPT, speed, 2.5);
  assert.equal(paramsDiffer(changed, SCRIPT), true);
  // The saved text comes back exactly, so the file is no longer "changed".
  assert.equal(resetParams(changed, SCRIPT), SCRIPT);
});

test("uniform version keeps line numbers", () => {
  const u = withUniforms(SCRIPT, parseParams(SCRIPT));
  const lines = u.split("\n");
  assert.equal(lines.length, SCRIPT.split("\n").length);
  assert.equal(lines[1], "uniform float speed;");
  assert.equal(lines[2], "uniform int layers;");
  assert.equal(lines[3], "uniform vec3 tint;");
});

test("windows line endings survive", () => {
  const crlf = SCRIPT.replace(/\n/g, "\r\n");
  const [speed] = parseParams(crlf);
  assert.equal(setParam(crlf, speed, 2).split("\n")[1], "float speed = 2.00; // @slider 0.1 3.0 Wind speed\r");
});

test("labels and colors", () => {
  assert.equal(labelFromName("starDensity"), "Star density");
  assert.equal(labelFromName("shooting_star_rate"), "Shooting star rate");
  assert.equal(toHex([1, 0.5, 0]), "#ff8000");
  assert.deepEqual(fromHex("#ff0000"), [1, 0, 0]);
});
