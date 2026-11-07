"use client";

import { usePathname } from "next/navigation";
import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";

/** Below this width the course tree is a menu that slides in; from it on, a column. Tailwind's `lg`. */
const COLUMN = "(min-width: 1024px)";

type Drawer = { open: boolean; show: () => void; close: () => void };
const DrawerContext = createContext<Drawer>({ open: false, show: () => {}, close: () => {} });

export const useNavDrawer = () => useContext(DrawerContext);

/** Whether the course menu is open, shared by the header's button and the sidebar. Going to another page closes it. */
export function NavDrawerProvider({ children }: { children: ReactNode }) {
  const pathname = usePathname() ?? "";
  // Open on one page only: a new address closes it, with no effect to run.
  const [openAt, setOpenAt] = useState<string | null>(null);
  const open = openAt === pathname;
  const show = useCallback(() => setOpenAt(pathname), [pathname]);
  const close = useCallback(() => setOpenAt(null), []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    // A window made wide enough has the column, and nothing left to close the menu with.
    const wide = window.matchMedia(COLUMN);
    wide.addEventListener("change", close);
    window.addEventListener("keydown", onKey);
    // The page behind the menu stays still while the menu scrolls.
    const root = document.documentElement;
    const before = root.style.overflow;
    root.style.overflow = "hidden";
    return () => {
      wide.removeEventListener("change", close);
      window.removeEventListener("keydown", onKey);
      root.style.overflow = before;
    };
  }, [open, close]);

  return <DrawerContext.Provider value={{ open, show, close }}>{children}</DrawerContext.Provider>;
}

/** The header's button for the course menu. Focus comes back to it when the menu closes. */
export function MenuButton() {
  const { open, show } = useNavDrawer();
  const button = useRef<HTMLButtonElement>(null);
  const wasOpen = useRef(false);
  useEffect(() => {
    if (wasOpen.current && !open) button.current?.focus({ preventScroll: true });
    wasOpen.current = open;
  }, [open]);
  return (
    <button
      ref={button}
      onClick={show}
      aria-label="Open the course menu"
      aria-expanded={open}
      aria-controls="course-nav"
      className="flex h-10 w-10 shrink-0 cursor-pointer items-center justify-center rounded-lg border border-line bg-face text-muted lg:hidden"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path d="M4 6.5h16M4 12h16M4 17.5h16" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
      </svg>
    </button>
  );
}
