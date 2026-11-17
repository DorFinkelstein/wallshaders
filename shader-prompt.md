# Wallshaders shader prompt

Copy everything inside the box below into an AI chat, replace `[describe shader]` at the bottom with what you want, and send it. Save the code you get back as a `.txt` or `.glsl` file and import it in Wallshaders (Library → Import).

```text
Write a GLSL fragment shader for Wallshaders, a Windows app that renders a shader as the live desktop wallpaper. The shader runs all day behind the user's windows, often on weak hardware and next to games, so it has to be cheap, calm and good to look at for hours.

FORMAT

- Shadertoy style. Define exactly this entry point and no main():
    void mainImage(out vec4 fragColor, in vec2 fragCoord)
- Do not write a #version line, a precision line, or declarations for the uniforms or the output. The app adds them.
- These uniforms already exist:
    vec3  iResolution   viewport size in pixels (z is 1.0)
    float iTime         seconds since the wallpaper started
    float iTimeDelta    seconds since the previous frame
    float iFrameRate    frames per second
    int   iFrame        frame counter
    vec4  iMouse        mouse position in pixels (xy)
    vec4  iDate         year, month, day, seconds since midnight
- There are no textures, no iChannel inputs, no buffers or multiple passes, and no audio. Everything must be computed in this one shader. Build noise from hash functions.
- One self-contained file, under 20,000 characters.

SETTINGS

The app shows a slider or colour picker for top-level constants marked with a comment. Give the shader 3 to 6 settings that are fun to change, such as speed, density, brightness, size and the main colours. Declare them at the top of the file, outside any function, one per line, exactly in this form:

    float speed = 1.0;                      // @slider 0.0 3.0 Speed
    int count = 12;                         // @slider 1 40 Number of lights
    vec3 skyColor = vec3(0.10, 0.12, 0.20); // @color Sky

- After @slider come the minimum, the maximum and then the label. After @color comes the label.
- The value must be a plain number (or vec3 of three plain numbers), never an expression.
- Never assign to a setting in the code, and do not use one where only a constant is allowed (such as an array size), so the app can change it while the shader runs.
- If you loop over an int setting, give the loop a constant upper bound and break out early: for (int i = 0; i < 40; i++) { if (i >= count) break; ... }
- The defaults must give the intended look.

COMPATIBILITY

The same code must compile as desktop GLSL 3.30 and as GLSL ES 3.00 (WebGL2), so:

- Write every float literal with a decimal point (1.0, not 1) and never mix int and float without an explicit cast such as float(i).
- Loops need a constant upper bound.
- Use only built-in functions that exist in both versions. No texture lookups, no dFdx/dFdy tricks that the picture depends on, no double type, no bit tricks on floats.
- Give every variable a value before reading it, and write to fragColor on every path, with alpha 1.0.
- Do not name anything after a built-in function or a reserved word (for example: sample, input, output, filter, half, smooth, common, active).

PERFORMANCE

- The wallpaper usually runs at 30 fps on a 1080p to 4K screen. Keep the work per pixel small: no raymarching with more than about 24 steps, no nested loops that total more than about 60 iterations per pixel, and at most 5 octaves of noise.
- Prefer cheap math (hashes, smoothstep, a few sin calls) to heavy loops.
- If the picture does not need to move, do not use iTime, iFrame, iMouse or iDate at all. The app then draws it once and uses no GPU afterwards.

LOOK

- It is a desktop background: icons and windows sit on top of it. Keep it fairly dark or muted, low in contrast, with no pure white areas and nothing that flashes, strobes or moves fast.
- Do not move the whole picture at once (no panning, rotating or zooming of the full scene). It makes people motion sick. Let individual things change inside a steady frame.
- Motion should be slow and should never visibly loop or repeat. Give separate elements their own timing by using unrelated periods.
- It must look right at any aspect ratio, including ultrawide. Correct for aspect ratio using iResolution, and do not let anything important depend on a fixed pixel size: scale sizes from iResolution.y.
- The shader must look complete with the mouse ignored. Mouse input is optional and may be turned off.
- Avoid visible banding in gradients: add a very small amount of hash noise (dither) to the final colour.
- Avoid visible grid patterns when placing things in cells: jitter positions, and fade each thing out before it reaches its cell edge so it is never cut off.
- Make it look hand made, not generic: uneven sizes and spacing, a restrained palette of a few related colours, small imperfections.

ANSWER

Reply with the complete shader in one code block, ready to save as a file. Start the file with a short comment giving the shader's name and one line about it. After the code block, add at most three sentences about what can be tuned and where. Do not leave out any part of the code or use placeholders.

THE SHADER I WANT

[describe shader]
```

## Tips

- Say what the scene is, the colours, the mood and how much movement you want. "A foggy pine forest at dawn, blue-grey, with slow drifting mist" works better than "a forest".
- If Wallshaders shows an error after importing, paste the error message back to the AI and ask it to fix the shader.
- To change the result, keep the chat going and ask for one or two changes at a time.
