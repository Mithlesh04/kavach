import { pathNodes, paths } from "@/lib/paths";

export const dynamic = "force-static";

// paths.json: each learning path's pages in order, by address, for the
// path bar and the sidebar, which run in the browser. Built with the site,
// so it always lists the pages that exist.
export function GET() {
  return Response.json({
    paths: paths.map((p) => ({ id: p.id, title: p.title, pages: pathNodes(p).map((n) => ({ href: n.href, title: n.title })) })),
  });
}
