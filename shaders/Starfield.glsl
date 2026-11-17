// Starfield — quiet twinkling stars over a charcoal night.

float starDensity = 22.0;                 // @slider 8.0 50.0 Star density
float twinkleSpeed = 1.0;                 // @slider 0.0 4.0 Twinkle speed
vec3 starColor = vec3(1.00, 0.86, 0.76);  // @color Stars
vec3 horizonColor = vec3(0.30, 0.15, 0.10); // @color Horizon glow

float hash(vec2 p) {
    p = fract(p * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

float stars(vec2 uv, float density, float t) {
    vec2 cell = floor(uv * density);
    vec2 local = fract(uv * density) - 0.5;
    float seed = hash(cell);
    if (seed < 0.82) return 0.0;
    vec2 offset = vec2(hash(cell + 1.7), hash(cell + 9.2)) - 0.5;
    float d = length(local - offset * 0.7);
    float twinkle = 0.6 + 0.4 * sin(t * (0.5 + seed * 2.0) + seed * 40.0);
    return smoothstep(0.09, 0.0, d) * twinkle;
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.y;
    float y = fragCoord.y / iResolution.y;

    vec3 col = mix(vec3(0.20, 0.15, 0.15), vec3(0.09, 0.09, 0.11), y);

    float drift = iTime * 0.004;
    float tw = iTime * twinkleSpeed;
    float s = stars(uv + vec2(drift, 0.0), starDensity, tw)
            + stars(uv + vec2(drift * 2.0, 3.0), starDensity * 38.0 / 22.0, tw * 1.3) * 0.6;
    col += s * starColor;

    // A faint terracotta glow on the horizon
    col += horizonColor * exp(-y * 5.0);

    fragColor = vec4(col, 1.0);
}
