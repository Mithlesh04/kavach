import { ImageResponse } from "next/og";
import { SITE_URL } from "@/lib/site";

export const dynamic = "force-static";
export const alt = "Kavach — safety you can read, speed you can verify";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

// The share card: rendered once at build time, no runtime anywhere.
export default function OgImage() {
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
          <svg width="72" height="72" viewBox="0 0 32 32">
            <rect x="4" y="4" width="5.2" height="24" rx="2" fill="#E6A23C" />
            <path d="M14 14.6 23.4 4h5.4L18.6 15.2Z" fill="#E6A23C" />
            <path d="M14 17.4 23.4 28h5.4L18.6 16.8Z" fill="#E6A23C" />
          </svg>
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
    size,
  );
}
