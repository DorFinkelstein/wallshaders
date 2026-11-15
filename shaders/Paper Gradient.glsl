// Paper Gradient — a still image: no animation, so it costs nothing after the first frame.
// (Wallshaders draws a shader only once when it never reads iTime, iFrame, iMouse or iDate.)

float hash(vec2 p) {
    return fract(sin(dot(p, vec2(41.3, 289.1))) * 45758.5453);
}

void mainImage(out vec4 fragColor, in vec2 fragCoord) {
    vec2 uv = fragCoord / iResolution.xy;

    vec3 col = mix(vec3(0.87, 0.58, 0.47), vec3(0.24, 0.22, 0.22), smoothstep(-0.1, 1.1, uv.y + uv.x * 0.25));

    // Soft light in one corner, grain like rough paper
    col += vec3(0.16, 0.11, 0.07) * exp(-length(uv - vec2(0.15, 0.1)) * 2.5);
    col += (hash(fragCoord) - 0.5) * 0.03;

    fragColor = vec4(col, 1.0);
}
