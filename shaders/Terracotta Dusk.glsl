// Terracotta Dusk — a slow, warm sky that drifts like evening haze.

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x),
               mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float t = iTime * 0.03;

    // Three octaves of soft cloud
    vec2 p = uv * vec2(2.0, 3.0);
    float n = noise(p + vec2(t, 0.0)) * 0.5
            + noise(p * 2.0 - vec2(t * 1.3, t)) * 0.3
            + noise(p * 4.0 + vec2(0.0, t * 2.0)) * 0.2;

    vec3 night      = vec3(0.17, 0.16, 0.17);
    vec3 clay       = vec3(0.55, 0.33, 0.27);
    vec3 terracotta = vec3(0.85, 0.56, 0.45);
    vec3 cream      = vec3(0.96, 0.84, 0.72);

    float h = uv.y + (n - 0.5) * 0.35;
    vec3 col = mix(cream, terracotta, smoothstep(0.0, 0.35, h));
    col = mix(col, clay, smoothstep(0.3, 0.65, h));
    col = mix(col, night, smoothstep(0.55, 1.05, h));

    fragColor = vec4(col, 1.0);
}
