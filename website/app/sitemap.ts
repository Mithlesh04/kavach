import type { MetadataRoute } from "next";
import { SITE_URL, visibleNodes } from "@/lib/site";

export const dynamic = "force-static";

const BASE = SITE_URL;

export default function sitemap(): MetadataRoute.Sitemap {
  return [
    { url: `${BASE}/`, priority: 1 },
    { url: `${BASE}/docs/benchmarks/`, priority: 0.9 },
    ...visibleNodes().map((n) => ({ url: `${BASE}${n.href}`, priority: n.sub ? 0.6 : 0.8 })),
  ];
}
