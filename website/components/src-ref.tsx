import versions from "@/data/versions.json";

/**
 * A source reference pinned to this docs version's tag. The build-time
 * checker (scripts/check-refs.mjs, with the sample runner) verifies the
 * file and symbol still live at this line for the tag, so references
 * can never rot.
 */
export function SrcRef({
  file,
  line,
  symbol,
  repo,
  label,
}: {
  file: string;
  line?: number;
  symbol?: string;
  /** Another project's repo (e.g. gcc) — ours when omitted. */
  repo?: { url: string; label: string };
  label?: string;
}) {
  const current = versions.versions.find((v) => v.id === versions.latest) ?? versions.versions[0];
  const base = repo?.url ?? `${versions.repo}/blob/${current.tag}`;
  const href = `${base}/${file}${line ? `#L${line}` : ""}`;
  const who = repo ? `theirs @${repo.label}` : `ours @${current.id}`;
  const text = label ?? `${who} · ${file.split("/").pop()}${line ? `:${line}` : ""}${symbol ? ` ${symbol}()` : ""}`;
  return (
    <a
      href={href}
      className="inline-block max-w-full rounded-[7px] border border-line bg-panel px-3 py-1.5 font-mono text-xs no-underline [overflow-wrap:anywhere]"
    >
      {text}
    </a>
  );
}
