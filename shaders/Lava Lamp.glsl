// Lava Lamp — a handful of slow blobs merging and drifting apart.

float speed = 1.0;                        // @slider 0.0 4.0 Speed
float blobSize = 1.0;                     // @slider 0.5 1.6 Blob size
vec3 blobColor = vec3(0.85, 0.56, 0.45);  // @color Blobs
vec3 glowColor = vec3(0.60, 0.33, 0.26);  // @color Glow

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    float t = iTime * 0.15 * speed;

    float field = 0.0;
    for (int i = 0; i < 6; i++) {
        float f = float(i);
        vec2 center = vec2(sin(t * (0.5 + f * 0.13) + f * 2.1) * 0.65,
                           cos(t * (0.4 + f * 0.17) + f * 1.3) * 0.38);
        float radius = (0.10 + 0.04 * sin(f * 3.7)) * blobSize;
        field += radius * radius / dot(uv - center, uv - center);
    }

    vec3 background = mix(vec3(0.16, 0.15, 0.16), vec3(0.22, 0.19, 0.19), uv.y + 0.5);
    vec3 highlight = min(blobColor + vec3(0.13, 0.24, 0.21), vec3(1.0));
    vec3 blob = mix(blobColor, highlight, smoothstep(1.0, 2.5, field));

    vec3 col = background + glowColor * smoothstep(0.3, 1.0, field) * 0.5;
    col = mix(col, blob, smoothstep(0.97, 1.03, field));

    fragColor = vec4(col, 1.0);
}
