<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState } from "@codemirror/state";
  import { keymap } from "@codemirror/view";
  import { indentWithTab } from "@codemirror/commands";
  import { cpp } from "@codemirror/lang-cpp";
  import { HighlightStyle, indentUnit, syntaxHighlighting } from "@codemirror/language";
  import { setDiagnostics, type Diagnostic } from "@codemirror/lint";
  import { tags as t } from "@lezer/highlight";
  import type { ShaderError } from "./glsl";

  interface Props {
    /** Identifies the open document. Each one keeps its own cursor and undo history. */
    docId: number;
    /** Text used the first time a document is shown. */
    source: string;
    errors: ShaderError[];
    onchange: (source: string) => void;
    onsave: () => void;
  }
  let { docId, source, errors, onchange, onsave }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  let shownId: number | null = null;
  const states = new Map<number, EditorState>();

  /** Drop the stored state of a closed document. */
  export function forget(id: number) {
    states.delete(id);
  }

  const theme = EditorView.theme(
    {
      "&": { height: "100%", backgroundColor: "var(--bg-2)", color: "var(--text)", fontSize: "13px" },
      "&.cm-focused": { outline: "none" },
      ".cm-scroller": { fontFamily: "var(--font-code)", lineHeight: "1.6" },
      ".cm-content": { padding: "12px 0", caretColor: "var(--accent-bright)" },
      ".cm-cursor": { borderLeftColor: "var(--accent-bright)", borderLeftWidth: "2px" },
      ".cm-gutters": { backgroundColor: "var(--bg-2)", color: "var(--text-faint)", border: "none", paddingLeft: "6px" },
      ".cm-activeLine": { backgroundColor: "rgba(255, 240, 230, 0.035)" },
      ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--accent)" },
      "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground": {
        backgroundColor: "rgba(217, 142, 115, 0.26)",
      },
      ".cm-selectionMatch": { backgroundColor: "rgba(217, 142, 115, 0.16)" },
      ".cm-matchingBracket, &.cm-focused .cm-matchingBracket": {
        backgroundColor: "rgba(217, 142, 115, 0.22)",
        outline: "none",
      },
      ".cm-foldPlaceholder": { backgroundColor: "var(--bg-3)", border: "none", color: "var(--text-dim)" },
      ".cm-tooltip": {
        backgroundColor: "var(--bg-0)",
        border: "1px solid var(--line)",
        borderRadius: "8px",
        overflow: "hidden",
      },
      ".cm-tooltip-autocomplete ul li[aria-selected]": { backgroundColor: "var(--accent-wash)", color: "var(--text)" },
      ".cm-panels": { backgroundColor: "var(--bg-1)", color: "var(--text)", borderTop: "1px solid var(--line)" },
      ".cm-textfield": { backgroundColor: "var(--bg-2)", border: "1px solid var(--line)", borderRadius: "6px" },
      ".cm-button": { backgroundImage: "none", backgroundColor: "var(--bg-3)", border: "none", borderRadius: "6px" },
      ".cm-diagnostic-error": { borderLeftColor: "var(--bad)" },
      ".cm-lintRange-error": {
        backgroundImage: "none",
        textDecoration: "underline wavy var(--bad)",
        textUnderlineOffset: "3px",
      },
    },
    { dark: true },
  );

  const highlight = HighlightStyle.define([
    { tag: [t.comment, t.lineComment, t.blockComment], color: "#857b74", fontStyle: "italic" },
    { tag: [t.keyword, t.controlKeyword, t.modifier, t.definitionKeyword], color: "#e09a80" },
    { tag: [t.typeName, t.standard(t.typeName)], color: "#d9b98a" },
    { tag: [t.number, t.bool, t.atom], color: "#e3c08f" },
    { tag: [t.string], color: "#b5c49a" },
    { tag: [t.function(t.variableName), t.function(t.definition(t.variableName))], color: "#f1cdb9" },
    { tag: [t.processingInstruction, t.meta, t.macroName], color: "#c59fb3" },
    { tag: [t.operator, t.punctuation, t.bracket], color: "#b7ada6" },
    { tag: [t.variableName, t.propertyName], color: "#ebe5df" },
  ]);

  function createState(text: string): EditorState {
    return EditorState.create({
      doc: text,
      extensions: [
        basicSetup,
        keymap.of([
          {
            key: "Mod-s",
            run: () => {
              onsave();
              return true;
            },
          },
          indentWithTab,
        ]),
        indentUnit.of("    "),
        cpp(),
        theme,
        syntaxHighlighting(highlight),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) onchange(u.state.doc.toString());
        }),
      ],
    });
  }

  function show(id: number) {
    if (!view || id === shownId) return;
    if (shownId !== null) states.set(shownId, view.state);
    view.setState(states.get(id) ?? createState(untrack(() => source)));
    shownId = id;
  }

  function applyErrors(list: ShaderError[]) {
    if (!view) return;
    const doc = view.state.doc;
    const diagnostics: Diagnostic[] = list.map((e) => {
      const line = doc.line(Math.min(Math.max(e.line, 1), doc.lines));
      return { from: line.from, to: Math.max(line.to, line.from), severity: "error", message: e.message };
    });
    view.dispatch(setDiagnostics(view.state, diagnostics));
  }

  onMount(() => {
    view = new EditorView({ parent: host, state: createState(untrack(() => source)) });
    shownId = untrack(() => docId);
    applyErrors(untrack(() => errors));
    return () => view?.destroy();
  });

  $effect(() => {
    show(docId);
    applyErrors(errors);
  });
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    user-select: text;
    -webkit-user-select: text;
  }
</style>
