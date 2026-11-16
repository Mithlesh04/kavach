import { ImageResponse } from "next/og";
import { Logo } from "@/components/logo";
import { LOGO } from "@/lib/site";

export const dynamic = "force-static";

// The logo as a large PNG, for what can't use icon.svg: search engines'
// logo signal (the Organization in the JSON-LD), the web manifest, and AI
// tools that show a site's logo. The colors match apple-icon.png.
export function GET() {
  return new ImageResponse(
    (
      <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center", background: "#1C1917" }}>
        <Logo size={LOGO.size * 0.8} color="#E8590C" />
      </div>
    ),
    { width: LOGO.size, height: LOGO.size },
  );
}
