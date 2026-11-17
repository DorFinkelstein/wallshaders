// Aurora — soft curtains of light swaying over a dark sky.

float speed = 1.0;                       // @slider 0.0 4.0 Speed
float brightness = 1.0;                  // @slider 0.2 2.5 Brightness
vec3 lowColor = vec3(0.90, 0.52, 0.40);  // @color Lower curtain
vec3 highColor = vec3(0.55, 0.75, 0.62); // @color Upper curtain

float hash(float n) {
    return fract(sin(n) * 43758.5453);
}

float noise(float x) {
    float i = floor(x);
    float f = fract(x);
    f = f * f * (3.0 - 2.0 * f);
    return mix(hash(i), hash(i + 1.0), f);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;
    float t = iTime * 0.08 * speed;

    vec3 col = mix(vec3(0.13, 0.12, 0.13), vec3(0.07, 0.07, 0.09), uv.y);

    for (int i = 0; i < 3; i++) {
        float f = float(i);
        float x = uv.x * (2.0 + f) + f * 13.0;
        float wave = noise(x + t * (1.0 + f * 0.4)) * 0.25 + noise(x * 2.7 - t) * 0.1;
        float center = 0.45 + f * 0.12 + wave;
        float d = uv.y - center;
        // Sharp lower edge, long fade upward
        float band = exp(-max(d, 0.0) * 6.0) * smoothstep(-0.05, 0.0, d);
        float rays = 0.7 + 0.3 * noise(uv.x * 40.0 + f * 5.0 + t * 3.0);
        vec3 tint = mix(lowColor, highColor, f / 2.0);
        col += tint * band * rays * 0.35 * brightness;
    }

    fragColor = vec4(col, 1.0);
}
