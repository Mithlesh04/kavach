/**
 * The memory-hop comparison: Kavach's dot makes one hop to the
 * key-and-value bucket; C++'s chases two pointers first. The eye gets
 * the "why" before any prose does.
 */
export function HopDiagram() {
  return (
    <div className="rounded-xl border border-line bg-panel px-6 py-5 max-sm:px-4">
      <div className="mb-1.5 flex flex-wrap items-center gap-x-3">
        <span aria-hidden="true" className="h-2.5 w-2.5 rounded-[3px]" style={{ background: "var(--ch-k)" }} />
        <span className="text-sm font-extrabold">Kavach</span>
        <span className="text-[12.5px] text-soft">hash → 16-byte bucket. Done.</span>
      </div>
      {/* The hops are measured in pixels, here and in their animation: on a narrow screen the track scrolls. */}
      <div className="overflow-x-auto">
      <div
        className="relative h-11 min-w-[560px]"
        role="img"
        aria-label="Animated: Kavach's lookup makes one hop straight to the key-and-value bucket"
      >
        <div
          className="absolute left-0 top-2.5 h-3.5 w-3.5 rounded-full"
          style={{ background: "var(--ch-k)", animation: "hop1 3s ease-in-out infinite" }}
        />
        <div
          className="absolute left-[236px] top-0.5 rounded-lg border-[1.5px] px-3 py-1.5 font-mono text-xs"
          style={{ borderColor: "var(--accent)", background: "var(--accent-soft)" }}
        >
          key · value
        </div>
        <span className="absolute right-0 top-2 text-[12.5px] font-bold" style={{ color: "var(--ok)" }}>
          1 memory touch
        </span>
      </div>
      </div>

      <div className="mb-1.5 mt-2.5 flex flex-wrap items-center gap-x-3 border-t border-dashed border-line-2 pt-3.5">
        <span aria-hidden="true" className="h-2.5 w-2.5 rounded-[3px]" style={{ background: "var(--ch-c)" }} />
        <span className="text-sm font-extrabold">C++ std::unordered_map</span>
        <span className="text-[12.5px] text-soft">hash → bucket array → chase the node pointer…</span>
      </div>
      {/* The hops are measured in pixels, here and in their animation: on a narrow screen the track scrolls. */}
      <div className="overflow-x-auto">
      <div
        className="relative h-11 min-w-[560px]"
        role="img"
        aria-label="Animated: C++'s lookup hops from the bucket array to a first node and then a second node"
      >
        <div
          className="absolute left-0 top-2.5 h-3.5 w-3.5 rounded-full"
          style={{ background: "var(--ch-c)", animation: "hop3 3s ease-in-out infinite" }}
        />
        <div className="absolute left-[104px] top-0.5 rounded-lg border-[1.5px] border-line px-3 py-1.5 font-mono text-xs text-muted">
          bucket*
        </div>
        <div className="absolute left-[206px] top-0.5 rounded-lg border-[1.5px] border-line px-3 py-1.5 font-mono text-xs text-muted">
          node*
        </div>
        <div
          className="absolute left-[308px] top-0.5 rounded-lg border-[1.5px] px-3 py-1.5 font-mono text-xs"
          style={{ borderColor: "var(--ch-c)" }}
        >
          key · value
        </div>
        <span className="absolute right-0 top-2 text-[12.5px] font-bold" style={{ color: "var(--bad)" }}>
          2–3 memory touches
        </span>
      </div>
      </div>
    </div>
  );
}
