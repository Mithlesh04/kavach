import type { NextConfig } from "next";

// A fully static site (decision 0033): no backend anywhere, so the
// export deploys unchanged to Vercel or GitHub Pages.
const nextConfig: NextConfig = {
  output: "export",
  // Folder/index.html URLs work on every static host (GitHub Pages too).
  trailingSlash: true,
  // A frozen docs version serves under /vX.Y/ (scripts/build-versions.mjs).
  basePath: process.env.SITE_BASE_PATH ?? "",
  // The browser needs it too, for the files it loads itself (search).
  env: { NEXT_PUBLIC_BASE_PATH: process.env.SITE_BASE_PATH ?? "" },
  images: { unoptimized: true },
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
