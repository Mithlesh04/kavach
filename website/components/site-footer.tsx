import project from "@/data/project.json";
import versions from "@/data/versions.json";

/** The footer of every page: who makes Kavach, where its source is, its license. */
export function SiteFooter({ width = "max-w-[1200px]" }: { width?: string }) {
  return (
    <footer className="mt-auto border-t border-line bg-panel">
      <div className={`mx-auto flex w-full ${width} flex-wrap items-center gap-x-6 gap-y-2 px-8 py-6 text-[13.5px] text-soft max-lg:px-4`}>
        <span className="font-bold text-muted">
          Kavach <span lang="hi">{project.nameHindi}</span>
        </span>
        <span>
          Created and led by{" "}
          <a href={project.lead.url} className="font-semibold">
            {project.lead.name}
          </a>
          , who writes this course
        </span>
        <span className="flex-1" />
        <a href={versions.repo}>GitHub</a>
        <a href={`${versions.repo}/discussions`}>Give feedback</a>
        <a href={`${versions.repo}/tree/main/docs/spec`}>Language spec</a>
        <span>Open source · Apache-2.0 or MIT</span>
      </div>
    </footer>
  );
}
