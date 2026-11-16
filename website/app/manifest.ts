import type { MetadataRoute } from "next";
import { DESCRIPTION, LOGO } from "@/lib/site";

export const dynamic = "force-static";

// The web manifest: the site's name and its large logo, which browsers,
// crawlers and AI tools read for the site's icon.
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "Kavach (कवच)",
    short_name: "Kavach",
    description: DESCRIPTION,
    start_url: "/",
    display: "browser",
    background_color: "#1C1917",
    theme_color: "#1C1917",
    icons: [
      { src: "/icon.svg", sizes: "any", type: "image/svg+xml" },
      { src: LOGO.path, sizes: `${LOGO.size}x${LOGO.size}`, type: "image/png" },
    ],
  };
}
