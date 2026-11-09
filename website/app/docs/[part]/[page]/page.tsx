import { DocPage } from "@/components/doc-page";
import { lessons } from "@/content/lessons";
import { findNode } from "@/lib/nav";
import { SITE_URL, visibleNodes } from "@/lib/site";

type Params = { params: Promise<{ part: string; page: string }> };

export function generateStaticParams() {
  return visibleNodes()
    .filter((n) => !n.sub)
    .map((n) => ({ part: n.part.slug, page: n.page.slug }));
}

export async function generateMetadata({ params }: Params) {
  const { part, page } = await params;
  const key = `${part}/${page}`;
  return {
    title: findNode(key)?.title ?? "Docs",
    description: lessons[key]?.lead,
    alternates: { canonical: `${SITE_URL}/docs/${key}/` },
  };
}

export default async function Page({ params }: Params) {
  const { part, page } = await params;
  return <DocPage nodeKey={`${part}/${page}`} />;
}
