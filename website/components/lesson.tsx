import Link from "next/link";
import { QuizBlock } from "@/components/quiz-block";
import { SmartBox } from "@/components/smart-box";
import type { LessonDoc } from "@/lib/chapter";
import { findNode, levelLabels, type CourseNode } from "@/lib/nav";
import { hasContent, smartFor, visibleNodes } from "@/lib/site";

const levelColor = { beginner: "text-ok", intermediate: "text-accent-text", advanced: "text-ch-c" };

/**
 * A one-idea page (decision 0047), one scroll: what it is, examples, "Try
 * it yourself", what Kavach does for you, "Go deeper", a quiz. Beginners
 * read the open parts; experienced readers open "Go deeper".
 */
export function Lesson({ node, doc, nextTitle }: { node: CourseNode; doc: LessonDoc; nextTitle?: string }) {
  const before = (doc.before ?? []).map((k) => findNode(k)).filter((n): n is CourseNode => !!n && hasContent(n.key));
  const subs = node.sub ? [] : visibleNodes().filter((n) => n.sub && n.page === node.page);
  const smart = smartFor(node.key);
  return (
    <div className="px-10 pb-10 max-md:px-5 max-sm:px-4">
      <p className="m-0 mb-3 flex flex-wrap gap-x-[18px] gap-y-1 text-[13px] text-soft">
        <span className={`font-bold ${levelColor[node.part.level]}`}>● {levelLabels[node.part.level]}</span>
        <span>{doc.minutes} min</span>
        {before.length > 0 && (
          <span>
            Before this:{" "}
            {before.map((b, i) => (
              <span key={b.key}>
                {i > 0 && ", "}
                <Link href={b.href}>{b.sub ? `${b.page.title}: ${b.title}` : b.title}</Link>
              </span>
            ))}
          </span>
        )}
        {node.page.spec !== "—" && !node.sub && <span>Spec {node.page.spec}</span>}
      </p>
      <p className={`m-0 max-w-[64ch] text-[17px] leading-relaxed text-muted min-[1920px]:text-[19px] ${doc.simple ? "mb-4" : "mb-6"}`}>{doc.lead}</p>
      {doc.simple && (
        <aside aria-label="In simple terms" className="mb-6 max-w-[64ch] rounded-r-lg border-l-4 border-accent bg-accent-soft px-4 py-3">
          <p className="m-0 mb-1 text-[12px] font-bold uppercase tracking-wider text-accent-text">In simple terms</p>
          <p className="m-0 text-[16px] leading-relaxed text-ink">{doc.simple}</p>
        </aside>
      )}

      <div className="lesson-body flex flex-col gap-5 text-base leading-[1.75] min-[1920px]:text-[17px]">{doc.body}</div>

      {subs.length > 0 && (
        <section aria-label="Sub-pages" className="mt-8">
          <h2 className="m-0 mb-3 text-lg font-extrabold">One page for each form</h2>
          <div className="grid grid-cols-[repeat(auto-fill,minmax(200px,1fr))] gap-2.5">
            {subs.map((s) => (
              <Link key={s.key} href={s.href} className="rounded-[10px] border border-line bg-panel px-4 py-3 text-[14.5px] font-semibold text-ink no-underline hover:border-[var(--soft)]">
                {s.title} →
              </Link>
            ))}
          </div>
        </section>
      )}

      {doc.tryIt && (
        <section aria-label="Try it yourself" className="mt-8 rounded-xl border border-line bg-panel px-6 py-5">
          <h2 className="m-0 mb-2 text-lg font-extrabold">Try it yourself</h2>
          <div className="mb-3 text-[15.5px] leading-relaxed">{doc.tryIt.task}</div>
          <details>
            <summary className="cursor-pointer text-sm font-bold text-accent-text">Show the answer</summary>
            <div className="mt-3 flex flex-col gap-3">{doc.tryIt.answer}</div>
          </details>
        </section>
      )}

      {smart.length > 0 && <SmartBox things={smart} />}

      {doc.deeper && (
        <details className="mt-8 rounded-xl border border-line bg-panel px-6 py-4">
          <summary className="cursor-pointer text-base font-extrabold">
            Go deeper <span className="text-sm font-semibold text-soft">the why, for readers who want it</span>
          </summary>
          <div className="lesson-body mt-3 flex flex-col gap-4 text-[15.5px] leading-[1.75]">{doc.deeper}</div>
        </details>
      )}

      {doc.quiz && doc.quiz.length > 0 && (
        <section aria-label="Quiz" className="mt-10">
          <h2 className="m-0 mb-3 text-lg font-extrabold">Check yourself</h2>
          <QuizBlock questions={doc.quiz} nextTitle={nextTitle} />
        </section>
      )}
    </div>
  );
}
