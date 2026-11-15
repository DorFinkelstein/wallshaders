// Warm Plasma — the classic plasma effect in a clay palette.

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    float t = iTime * 0.2;

    float v = sin(uv.x * 4.0 + t)
            + sin(uv.y * 3.0 - t * 0.8)
            + sin((uv.x + uv.y) * 3.0 + t * 0.6)
            + sin(length(uv * 5.0) - t * 1.2);
    v = v * 0.125 + 0.5;

    vec3 a = vec3(0.19, 0.17, 0.18);
    vec3 b = vec3(0.62, 0.38, 0.31);
    vec3 c = vec3(0.90, 0.63, 0.50);
    vec3 col = mix(a, b, smoothstep(0.15, 0.55, v));
    col = mix(col, c, smoothstep(0.50, 0.90, v));

    fragColor = vec4(col, 1.0);
}
