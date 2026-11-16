import { ImageResponse } from "next/og";
import { Logo } from "@/components/logo";
import { CARD, SITE_URL } from "@/lib/site";

export const dynamic = "force-static";

// The share card, rendered once at build time, no runtime anywhere. It is
// a route named share-card.png, not Next's opengraph-image file, so the
// export writes a real .png file: a static host then serves it as
// image/png, which link previews and crawlers need (an extensionless
// file is served as application/octet-stream, and they drop it).
export function GET() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          padding: "80px",
          background: "#171412",
          color: "#f3efe7",
          fontFamily: "sans-serif",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 24 }}>
          <Logo size={72} color="#E6A23C" />
          <div style={{ fontSize: 56, fontWeight: 800 }}>Kavach</div>
        </div>
        <div style={{ fontSize: 64, fontWeight: 800, marginTop: 48, lineHeight: 1.15 }}>
          Safety you can read.
        </div>
        <div style={{ fontSize: 64, fontWeight: 800, lineHeight: 1.15 }}>Speed you can verify.</div>
        <div style={{ fontSize: 28, color: "#c9c2b6", marginTop: 40 }}>
          {`Every claim links to the line of code and the measurement behind it · ${SITE_URL.replace(/^https?:\/\//, "")}`}
        </div>
      </div>
    ),
    { width: CARD.width, height: CARD.height },
  );
}
