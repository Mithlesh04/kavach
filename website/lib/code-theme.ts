// The one Shiki theme for Kavach code, shared by the build-time
// highlighter (components/code-block.tsx) and the playground's in-browser
// one. Token colors are fixed hexes because code panels stay dark in both
// themes; they mirror the --tok-* tokens in globals.css.
export const kavachTheme = {
  name: "kavach-dark",
  type: "dark" as const,
  colors: { "editor.background": "#00000000", "editor.foreground": "#e7e0d4" },
  tokenColors: [
    // `self` and a contract's `result` are words of the language too, so they read like keywords.
    { scope: ["keyword", "storage", "keyword.control", "variable.language"], settings: { foreground: "#e6a23c" } },
    // `const` apart from `let`: a value that never changes reads differently (decision 0034).
    { scope: ["variable.other.constant"], settings: { foreground: "#56c2a6" } },
    { scope: ["entity.name.function", "support.function", "entity.name.type", "support.type", "entity.name.label"], settings: { foreground: "#8fc1ea" } },
    { scope: ["string"], settings: { foreground: "#9bb97f" } },
    // A unit after a number (`5.seconds`) reads with its number as one value, as `px` does in CSS.
    { scope: ["constant.numeric", "constant.language", "keyword.other.unit"], settings: { foreground: "#d9a0c7" } },
    { scope: ["comment"], settings: { foreground: "#857c6e" } },
    // `///` apart from `//`: a doc comment on a field is sent to the model by ask<T> (spec 8.4).
    { scope: ["comment.line.documentation"], settings: { foreground: "#4fa8a8" } },
    // Inside "{…}" in a string: braces like keywords, the code in them like code.
    { scope: ["punctuation.definition.template-expression"], settings: { foreground: "#e6a23c" } },
    { scope: ["meta.embedded"], settings: { foreground: "#e7e0d4" } },
    { scope: ["constant.character.escape"], settings: { foreground: "#d9a0c7" } },
    { scope: ["invalid"], settings: { foreground: "#f08a8a" } },
  ],
};
