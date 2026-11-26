import Link from "next/link";
import { PathBar } from "./path-bar";
import { notFound } from "next/navigation";
import { pageScenes } from "@/components/explain/scene-pages";
import { sceneStructured } from "@/components/explain/scene-text";
import { Lesson } from "@/components/lesson";
import { lessons } from "@/content/lessons";
import project from "@/data/project.json";
import { aliases } from "@/lib/aliases";
import { findNode } from "@/lib/nav";
import { quizStructured } from "@/lib/quiz-text";
import { CARD, PUBLISHER, SITE_URL, neighbors } from "@/lib/site";

/** One course page or sub-page: a one-idea lesson (decision 0047). */
export function DocPage({ nodeKey }: { nodeKey: string }) {
  const node = findNode(nodeKey);
  const lesson = lessons[nodeKey];
  if (!node || !lesson) notFound();
  const { prev, next } = neighbors(nodeKey);
  const title = node.sub ? `${node.page.title}: ${node.sub.title}` : node.title;
  const site = SITE_URL;
  // What search engines and AI crawlers read: this page as an article of the course, and where it sits in it.
  const structured = [
    {
      "@context": "https://schema.org",
      "@type": "TechArticle",
      headline: title,
      description: lesson.lead,
      url: `${site}${node.href}`,
      inLanguage: "en",
      timeRequired: `PT${lesson.minutes}M`,
      author: { "@type": "Person", name: project.lead.name, url: project.lead.url, sameAs: [project.lead.linkedin] },
      publisher: PUBLISHER,
      image: `${SITE_URL}${CARD.path}`,
      isPartOf: { "@type": "Course", name: "The Kavach tutorial", url: `${site}/docs/start/kavach-home/` },
      about: { "@type": "ComputerLanguage", name: "Kavach" },
    },
    {
      "@context": "https://schema.org",
      "@type": "BreadcrumbList",
      itemListElement: [
        { name: node.part.title, url: `${site}${node.part.pages[0] ? `/docs/${node.part.slug}/${node.part.pages[0].slug}/` : "/docs/"}` },
        { name: node.page.title, url: `${site}/docs/${node.part.slug}/${node.page.slug}/` },
        ...(node.sub ? [{ name: node.sub.title, url: `${site}${node.href}` }] : []),
      ].map((item, i) => ({ "@type": "ListItem", position: i + 1, name: item.name, item: item.url })),
    },
    // The quiz runs in the browser; its questions and answers, for readers that don't run it.
    ...(lesson.quiz?.length ? [quizStructured(title, `${site}${node.href}`, lesson.quiz)] : []),
    // The step-by-step animations run in the browser; each one's steps, as text.
    ...pageScenes(lesson).map((s) => sceneStructured(s, `${site}${node.href}`)),
  ];

  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(structured).replace(/</g, "\\u003c") }} />
      <div className="px-10 pt-[30px] max-md:px-5 max-sm:px-4 max-sm:pt-5" data-pagefind-meta={`title:${title}`}>
        <nav aria-label="Breadcrumb" className="m-0 mb-1 text-[12.5px] font-bold text-soft">
          Part {node.part.n} · {node.part.title} <span aria-hidden="true">›</span>{" "}
          {node.sub ? (
            <>
              <Link href={`/docs/${node.part.slug}/${node.page.slug}/`} className="text-soft">
                {node.page.title}
              </Link>{" "}
              <span aria-hidden="true">›</span>{" "}
            </>
          ) : null}
          <span aria-current="page" className="font-semibold text-ink">{node.title}</span>
        </nav>
        <h1 className="m-0 mb-2 text-[38px] font-extrabold leading-tight tracking-tight max-md:text-3xl max-sm:text-[26px] min-[1920px]:text-[46px]">{title}</h1>
        {aliases[nodeKey] && (
          <p className="sr-only" data-pagefind-weight="0.4">
            Also known as: {aliases[nodeKey]}
          </p>
        )}
      </div>

      <Lesson node={node} doc={lesson} nextTitle={next?.title} />

      <PathBar />
      <nav aria-label="Previous and next" className="flex flex-wrap justify-between gap-4 px-10 pb-12 max-md:px-5 max-sm:px-4">
        {prev ? (
          <Link href={prev.href} className="rounded-lg border border-line px-4 py-2.5 text-sm font-bold text-muted no-underline">
            ← {prev.sub ? `${prev.page.title}: ${prev.title}` : prev.title}
          </Link>
        ) : (
          <span />
        )}
        {next ? (
          <Link href={next.href} className="rounded-lg bg-inv-bg px-4 py-2.5 text-sm font-bold text-inv-ink no-underline">
            Next: {next.sub ? `${next.page.title}: ${next.title}` : next.title} →
          </Link>
        ) : (
          <span />
        )}
      </nav>
    </>
  );
}
