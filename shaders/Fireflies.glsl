// Fireflies — warm lights wandering through the dark.

float hash(float n) {
    return fract(sin(n * 12.9898) * 43758.5453);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.y;
    float aspect = iResolution.x / iResolution.y;
    float t = iTime * 0.12;

    vec3 col = mix(vec3(0.16, 0.14, 0.14), vec3(0.10, 0.10, 0.11), fragCoord.y / iResolution.y);

    for (int i = 0; i < 14; i++) {
        float f = float(i);
        vec2 home = vec2(hash(f) * aspect, hash(f + 31.0));
        vec2 wander = vec2(sin(t * (1.0 + hash(f + 7.0)) + f * 4.0),
                           cos(t * (0.8 + hash(f + 3.0)) + f * 2.0)) * 0.12;
        float d = length(uv - home - wander);
        float pulse = 0.5 + 0.5 * sin(iTime * (0.6 + hash(f + 11.0)) + f * 9.0);
        float size = 0.004 + 0.004 * hash(f + 5.0);
        col += vec3(1.0, 0.72, 0.48) * (size / (d + 0.002)) * pulse * 0.35;
    }

    fragColor = vec4(col, 1.0);
}
