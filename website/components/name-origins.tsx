import Link from "next/link";
import { Fragment, type ReactNode } from "react";
import { C } from "@/components/inline-code";
import type { Origin } from "@/lib/name-origins";

// Under a code window: where each name in the code comes from. The
// standard types it uses with no `use` are said at once, since nothing in
// the code shows their module; the rest opens on a click.

const where = (at?: { file?: string; line: number }) => (at?.file ? `${at.file}, line ${at.line}` : `line ${at?.line}`);

function list(items: ReactNode[]) {
  return items.map((item, i) => (
    <Fragment key={i}>
      {i === 0 ? "" : i === items.length - 1 ? " and " : ", "}
      {item}
    </Fragment>
  ));
}

const named = (o: Origin) => (o.href ? <Link href={o.href}><C>{o.name}</C></Link> : <C>{o.name}</C>);

function Row({ head, children }: { head: string; children: ReactNode }) {
  return (
    <li>
      <span className="text-ink">{head}:</span> {children}
    </li>
  );
}

export function NameOrigins({ origins }: { origins: Origin[] }) {
  if (!origins.length) return null;
  const of = (kind: Origin["kind"]) => origins.filter((o) => o.kind === kind);
  const unseen = of("std").filter((o) => !o.used);
  const modules = [...new Set(unseen.map((o) => o.module!))];
  return (
    <div className="mt-2 flex flex-col gap-1.5 text-[14px] leading-relaxed text-muted">
      {modules.length ? (
        <p className="m-0">
          From the standard library, with no <C>use</C> needed:{" "}
          {modules.map((m, i) => (
            <Fragment key={m}>
              {i > 0 ? "; " : ""}
              {list(unseen.filter((o) => o.module === m).map((o) => <Fragment key={o.name}>{named(o)}</Fragment>))} from <C>{m}</C>
            </Fragment>
          ))}
          .
        </p>
      ) : null}
      <details>
        <summary className="cursor-pointer">Where each name comes from: {origins.map((o) => o.name).join(", ")}</summary>
        <ul className="m-0 mt-1.5 flex list-disc flex-col gap-1 pl-5">
          {of("own").length ? (
            <Row head="Declared in this code">{list(of("own").map((o) => <Fragment key={o.name}><C>{o.name}</C> ({where(o.at)})</Fragment>))}</Row>
          ) : null}
          {of("variant").length ? (
            <Row head="Variants">
              {list(of("variant").map((o) => <Fragment key={o.name}><C>{o.name}</C>, a variant of <C>{o.union}</C> ({where(o.at)})</Fragment>))}
              {" "}(<Link href="/docs/types/unions/">what a variant is</Link>)
            </Row>
          ) : null}
          {of("std").length ? (
            <Row head="From the standard library">
              {[...new Set(of("std").map((o) => o.module!))].map((m, i) => (
                <Fragment key={m}>
                  {i > 0 ? "; " : ""}
                  {list(of("std").filter((o) => o.module === m).map((o) => <Fragment key={o.name}>{named(o)}</Fragment>))} from <C>{m}</C>
                  {of("std").some((o) => o.module === m && !o.used) ? <>, with no <C>use</C> needed</> : null}
                </Fragment>
              ))}
            </Row>
          ) : null}
          {of("builtin").length ? <Row head="Built in, in every program">{list(of("builtin").map((o) => <Fragment key={o.name}>{named(o)}</Fragment>))}</Row> : null}
          {of("capability").length ? <Row head="Built-in capabilities">{list(of("capability").map((o) => <Fragment key={o.name}>{named(o)}</Fragment>))}</Row> : null}
        </ul>
      </details>
    </div>
  );
}
