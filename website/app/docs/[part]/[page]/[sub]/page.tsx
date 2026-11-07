import { DocPage } from "@/components/doc-page";
import { lessons } from "@/content/lessons";
import { findNode } from "@/lib/nav";
import { SITE_URL, visibleNodes } from "@/lib/site";

type Params = { params: Promise<{ part: string; page: string; sub: string }> };

export function generateStaticParams() {
  return visibleNodes()
    .filter((n) => n.sub)
    .map((n) => ({ part: n.part.slug, page: n.page.slug, sub: n.sub!.slug }));
}

export async function generateMetadata({ params }: Params) {
  const { part, page, sub } = await params;
  const key = `${part}/${page}/${sub}`;
  const node = findNode(key);
  return {
    title: node ? `${node.page.title}: ${node.title}` : "Docs",
    description: lessons[key]?.lead,
    alternates: { canonical: `${SITE_URL}/docs/${key}/` },
  };
}

export default async function Page({ params }: Params) {
  const { part, page, sub } = await params;
  return <DocPage nodeKey={`${part}/${page}/${sub}`} />;
}
