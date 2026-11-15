import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { TipLayer } from "@/components/tip-layer";
import project from "@/data/project.json";
import { DESCRIPTION, SITE_URL } from "@/lib/site";
import "./globals.css";

const inter = Inter({ variable: "--font-inter", subsets: ["latin"] });
const mono = JetBrains_Mono({ variable: "--font-jetbrains", subsets: ["latin"] });

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: { default: "Kavach (कवच): a compiled programming language, safe for AI agents", template: "%s · Kavach" },
  authors: [{ name: project.lead.name, url: project.lead.url }],
  creator: project.lead.name,
  description: DESCRIPTION,
  keywords: ["Kavach", "KavachLang", "Kavach programming language", "programming language", "compiled language", "AI agents", "capabilities"],
  openGraph: { type: "website", siteName: "Kavach (कवच)", locale: "en", description: DESCRIPTION },
  twitter: { card: "summary_large_image" },
};

// Runs before paint so a forced theme never flashes the wrong colors. It
// must stay a plain inline script: next/script's beforeInteractive only
// queues it for later. In `next dev`, React warns "Encountered a script
// tag" when hot reload re-renders this layout; that copy never needs to
// run, because the server-rendered one already did.
const themeInit = `try{var t=localStorage.getItem("kavach-theme");if(t==="light"||t==="dark")document.documentElement.dataset.theme=t}catch(e){}`;

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${inter.variable} ${mono.variable} h-full antialiased`} suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeInit }} />
      </head>
      <body className="min-h-full flex flex-col">
        <a
          href="#main"
          className="sr-only z-50 rounded-md bg-inv-bg px-4 py-2 font-bold text-inv-ink focus:not-sr-only focus:fixed focus:left-4 focus:top-4"
        >
          Skip to content
        </a>
        {children}
        <TipLayer />
      </body>
    </html>
  );
}
