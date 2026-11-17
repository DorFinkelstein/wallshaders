// Soft Hills — layered hills rolling slowly under a warm sky.

float speed = 1.0;                        // @slider 0.0 4.0 Speed
float sunSize = 1.0;                      // @slider 0.0 2.5 Sun size
vec3 skyLow = vec3(0.95, 0.80, 0.68);     // @color Sky near the hills
vec3 skyHigh = vec3(0.80, 0.52, 0.42);    // @color Sky above

float hill(float x, float seed, float t) {
    return sin(x * 1.7 + seed + t) * 0.5
         + sin(x * 3.1 + seed * 2.3 - t * 0.7) * 0.25
         + sin(x * 6.3 + seed * 4.1 + t * 0.4) * 0.1;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float x = uv.x * iResolution.x / iResolution.y;
    float t = iTime * 0.05 * speed;

    vec3 col = mix(skyLow, skyHigh, uv.y);

    // Sun
    float sun = length(vec2(x - 1.25, uv.y - 0.72));
    float r = 0.10 * sunSize;
    col = mix(vec3(1.0, 0.93, 0.82), col, smoothstep(r, r + 0.01, sun));
    col += vec3(0.25, 0.14, 0.08) * exp(-sun * 5.0) * min(sunSize, 1.0);

    // Five ridges, far to near
    for (int i = 0; i < 5; i++) {
        float f = float(i);
        float base = 0.52 - f * 0.10;
        float h = base + hill(x * (1.0 + f * 0.25), f * 7.3, t * (1.0 + f * 0.6)) * (0.05 + f * 0.012);
        vec3 ridge = mix(vec3(0.72, 0.47, 0.39), vec3(0.20, 0.18, 0.19), f / 4.0);
        col = mix(col, ridge, smoothstep(h + 0.003, h - 0.003, uv.y));
    }

    fragColor = vec4(col, 1.0);
}
