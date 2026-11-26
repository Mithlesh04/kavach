import { loadProject } from "@/lib/samples";

// What every guide shows from its one project (content/samples/guides/
// <name>.project): its files, a piece of a file cut from the checked code,
// and each command's output cut from the checked session its `runs` file
// tells. Nothing a guide shows is typed by hand.

export function guideProject(name: string, files: string[]) {
  const project = loadProject(`guides/${name}`, files);
  const file = (p: string) => {
    const f = project.files.find((x) => x.path === p);
    if (!f) throw new Error(`guides/${name}: no file ${p} among ${files.join(", ")}`);
    return f.code;
  };
  return {
    file,
    /**
     * The lines of file `p` from the one holding `start` to the first `}`
     * at the start of a line, with the doc comments above it.
     */
    piece(p: string, start: string): string {
      const lines = file(p).split("\n");
      let from = lines.findIndex((l) => l.includes(start));
      if (from < 0) throw new Error(`guides/${name}: no line of ${p} holds ${JSON.stringify(start)}`);
      while (from > 0 && lines[from - 1].startsWith("///")) from--;
      const to = lines.findIndex((l, i) => i > from && l === "}");
      return lines.slice(from, to + 1).join("\n");
    },
    /** What `kavach <command>` printed in the checked session, with its command line. */
    said(command: string): string {
      const block = (project.output ?? "").split(/^(?=\$ kavach )/m).find((b) => b.startsWith(`$ kavach ${command}\n`));
      if (!block) throw new Error(`guides/${name}: the session has no "kavach ${command}"`);
      return block.trimEnd();
    },
  };
}
