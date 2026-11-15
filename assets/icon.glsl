// Source for the app icon. Rendered with:
//   ws-renderer --headless assets/icon.glsl --out assets/icon.png --size 1024x1024

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;

    vec3 col = mix(vec3(0.93, 0.72, 0.60), vec3(0.83, 0.52, 0.41), uv.y);

    float sun = length(uv - vec2(0.64, 0.66));
    col = mix(vec3(1.0, 0.94, 0.84), col, smoothstep(0.150, 0.156, sun));

    for (int i = 0; i < 3; i++) {
        float f = float(i);
        float h = 0.46 - f * 0.14 + sin(uv.x * (3.5 + f * 1.4) + f * 2.2 + 0.6) * 0.06;
        vec3 ridge = mix(vec3(0.62, 0.40, 0.34), vec3(0.20, 0.19, 0.19), f / 2.0);
        col = mix(col, ridge, smoothstep(h + 0.004, h - 0.004, uv.y));
    }

    fragColor = vec4(col, 1.0);
}
