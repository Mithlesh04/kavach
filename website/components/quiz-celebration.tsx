/**
 * The end of a quiz: a ribbon with the first-try score, and a burst of
 * confetti when every answer was right the first time. Pure CSS animation
 * (keyframes in globals.css), so it costs nothing until it shows; people
 * who ask their system for less motion see the ribbon without the burst.
 */

const colors = ["var(--ch-k)", "var(--ch-c)", "var(--ch-p)", "var(--ok)", "var(--accent)"];

// Fixed pieces, spread evenly round a circle: the same burst every time.
const pieces = Array.from({ length: 28 }, (_, i) => {
  const angle = (i / 28) * Math.PI * 2;
  const reach = 90 + (i % 4) * 28;
  return {
    dx: `${Math.round(Math.cos(angle) * reach * 1.6)}px`,
    dy: `${Math.round(Math.sin(angle) * reach - 40)}px`,
    rot: `${(i * 47) % 360}deg`,
    color: colors[i % colors.length],
    delay: `${(i % 5) * 0.03}s`,
    round: i % 3 === 0,
  };
});

export function QuizCelebration({ score, total, nextTitle }: { score: number; total: number; nextTitle?: string }) {
  const perfect = score === total;
  return (
    <div className="relative mt-4 flex justify-center" role="status">
      {perfect && (
        <div aria-hidden="true" className="pointer-events-none absolute left-1/2 top-1/2">
          {pieces.map((p, i) => (
            <span
              key={i}
              className="absolute block"
              style={
                {
                  width: p.round ? 8 : 6,
                  height: p.round ? 8 : 12,
                  borderRadius: p.round ? "50%" : 2,
                  background: p.color,
                  "--dx": p.dx,
                  "--dy": p.dy,
                  "--rot": p.rot,
                  animation: `confetti 1.3s cubic-bezier(0.15, 0.7, 0.3, 1) ${p.delay} both`,
                } as React.CSSProperties
              }
            />
          ))}
        </div>
      )}
      <div
        className={`quiz-ribbon relative px-9 py-2.5 text-center ${perfect ? "bg-accent text-bg" : "bg-inv-bg text-inv-ink"}`}
        style={{ animation: "ribbon-in 0.45s cubic-bezier(0.2, 1.4, 0.4, 1) both" }}
      >
        <span className="block text-[15px] font-extrabold tracking-tight">
          {score} / {total} {perfect ? "all right on the first try!" : "right on the first try"}
        </span>
        <span className="block text-[12.5px] font-semibold opacity-80">
          {perfect
            ? nextTitle
              ? `You've got this one. Next up: ${nextTitle}`
              : "You've got this one."
            : `You got there. Read the page once more, then go for ${total} / ${total}.`}
        </span>
      </div>
    </div>
  );
}
