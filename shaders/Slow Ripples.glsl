// Slow Ripples — rings spreading across still water at dusk.

float speed = 1.0;                       // @slider 0.0 4.0 Speed
float rings = 22.0;                      // @slider 8.0 50.0 Ring spacing
vec3 rippleColor = vec3(0.83, 0.54, 0.44); // @color Ripples

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = (fragCoord - 0.5 * iResolution.xy) / iResolution.y;
    float t = iTime * 0.4 * speed;

    float h = 0.0;
    for (int i = 0; i < 3; i++) {
        float f = float(i);
        vec2 source = vec2(sin(f * 2.4 + 1.0) * 0.6, cos(f * 1.7) * 0.3);
        float d = length(uv - source);
        h += sin(d * rings - t * (1.5 + f * 0.3)) * exp(-d * 2.2);
    }
    h /= 3.0;

    vec3 deep = vec3(0.17, 0.17, 0.19);
    vec3 col = mix(deep, vec3(0.30, 0.25, 0.25), uv.y + 0.5);
    col += rippleColor * smoothstep(0.05, 0.35, h) * 0.45;
    col -= 0.04 * smoothstep(-0.05, -0.3, h);

    fragColor = vec4(col, 1.0);
}
