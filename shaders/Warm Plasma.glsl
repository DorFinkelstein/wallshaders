// Warm Plasma — the classic plasma effect in a clay palette.

float speed = 1.0;                        // @slider 0.0 4.0 Speed
float scale = 1.0;                        // @slider 0.3 3.0 Pattern size
vec3 midColor = vec3(0.62, 0.38, 0.31);   // @color Middle
vec3 highColor = vec3(0.90, 0.63, 0.50);  // @color Highlights

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y / scale;
    float t = iTime * 0.2 * speed;

    float v = sin(uv.x * 4.0 + t)
            + sin(uv.y * 3.0 - t * 0.8)
            + sin((uv.x + uv.y) * 3.0 + t * 0.6)
            + sin(length(uv * 5.0) - t * 1.2);
    v = v * 0.125 + 0.5;

    vec3 a = vec3(0.19, 0.17, 0.18);
    vec3 col = mix(a, midColor, smoothstep(0.15, 0.55, v));
    col = mix(col, highColor, smoothstep(0.50, 0.90, v));

    fragColor = vec4(col, 1.0);
}
