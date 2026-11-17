// Source for the app icon. Rendered with:
//   ws-renderer --headless assets/icon.glsl --out assets/icon.png --size 1024x1024
//
// A charcoal tile holding three flowing ribbons of light in terracotta,
// stacked like a wallpaper seen through a window, with one small spark.

float roundedBox(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 p = (fragCoord - 0.5 * iResolution.xy) / iResolution.y; // -0.5..0.5
    float px = 1.5 / iResolution.y;

    // Tile
    float tile = roundedBox(p, vec2(0.46), 0.11);
    float tileMask = smoothstep(px, -px, tile);
    vec3 col = mix(vec3(0.13, 0.12, 0.12), vec3(0.20, 0.18, 0.17), p.y + 0.5);

    // Ribbons, clipped to an inner rounded window
    float win = roundedBox(p, vec2(0.36), 0.07);
    float winMask = smoothstep(px, -px, win);
    vec3 ribbonCol[3] = vec3[3](
        vec3(0.97, 0.78, 0.64),
        vec3(0.85, 0.56, 0.45),
        vec3(0.62, 0.36, 0.30)
    );
    for (int i = 0; i < 3; i++) {
        float f = float(i);
        float centre = 0.13 - f * 0.15 + sin(p.x * 6.0 + f * 0.9 + 0.4) * 0.07;
        float half_w = 0.045 + 0.012 * sin(p.x * 4.0 + f * 1.7);
        float d = abs(p.y - centre) - half_w;
        float m = smoothstep(px, -px, d) * winMask;
        col = mix(col, ribbonCol[i], m);
    }

    // Spark, top left
    vec2 s = p - vec2(-0.20, 0.25);
    float star = abs(s.x) * abs(s.y) * 900.0 + length(s) * 18.0;
    float spark = smoothstep(1.0, 0.6, star);
    col = mix(col, vec3(1.0, 0.95, 0.86), spark);

    fragColor = vec4(col * tileMask, tileMask);
}
