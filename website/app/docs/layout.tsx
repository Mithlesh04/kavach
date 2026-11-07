import Link from "next/link";
import { Logo } from "@/components/logo";
import { SiteFooter } from "@/components/site-footer";
import project from "@/data/project.json";
import { MenuButton, NavDrawerProvider } from "@/components/nav-drawer";
import { PageToc } from "@/components/page-toc";
import { Sidebar } from "@/components/sidebar";
import { SearchButton } from "@/components/search-button";
import { ThemeToggle } from "@/components/theme-toggle";
import { VersionPicker } from "@/components/version-picker";
import versions from "@/data/versions.json";
import { sidebarTree } from "@/lib/site";

/** How wide the shell may grow: the column, the text and "On this page" side by side, a little wider on a very large screen. */
const SHELL = "max-w-[1440px] min-[1920px]:max-w-[1680px]";

export default function DocsLayout({ children }: { children: React.ReactNode }) {
  return (
    <NavDrawerProvider>
    <div className="flex min-h-screen flex-col">
      {/* Below the column's width the header stays in reach: it holds the way to the course menu. */}
      <header data-pagefind-ignore className="relative z-30 border-b border-line bg-bg max-lg:sticky max-lg:top-0">
        <div className={`mx-auto flex min-h-[60px] w-full ${SHELL} items-center gap-x-5 px-7 py-2 max-lg:gap-x-3 max-lg:px-4 max-sm:gap-x-2 max-sm:px-3`}>
          <MenuButton />
          <Link href="/" className="flex shrink-0 items-center gap-2.5 text-lg font-extrabold tracking-tight text-ink no-underline max-sm:gap-1.5">
            <Logo />
            Kavach
            <span lang="hi" className="text-[0.8em] font-semibold text-soft max-sm:hidden">
              {project.nameHindi}
            </span>
          </Link>
          <div className="max-[359px]:hidden">
            <VersionPicker />
          </div>
          <nav aria-label="Main" className="flex gap-[18px] text-sm font-semibold max-md:hidden">
            <Link href="/docs/start/kavach-home/" className="font-bold text-ink no-underline">
              Learn
            </Link>
            <Link href="/docs/benchmarks/" className="text-muted no-underline">
              Benchmarks
            </Link>
            <a href={versions.repo} className="text-muted no-underline">
              GitHub
            </a>
          </nav>
          <div className="flex-1" />
          <SearchButton />
          <ThemeToggle />
        </div>
      </header>
      {/* min-w-0: the widest table or code line must scroll in its own box, never widen the page. */}
      <div className={`mx-auto box-border flex w-full min-w-0 ${SHELL} flex-1`}>
        <Sidebar
          tree={sidebarTree()}
          links={[
            { title: "Learn", href: "/docs/start/kavach-home/" },
            { title: "Benchmarks", href: "/docs/benchmarks/" },
            { title: "GitHub", href: versions.repo },
          ]}
        />
        <main id="main" className="mx-auto w-full min-w-0 max-w-[880px] flex-1 min-[1920px]:max-w-[1120px]">{children}</main>
        <PageToc />
      </div>
      <SiteFooter width={SHELL} />
    </div>
    </NavDrawerProvider>
  );
}
