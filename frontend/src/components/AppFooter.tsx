/* Developed with care by FACRF - https://github.com/facrf */
import { version } from "../../package.json";

export function AppFooter() {
  return (
    <footer className="flex flex-wrap items-center justify-center gap-x-2 gap-y-1 px-4 py-5 text-center text-[11px] text-slate-500">
      <span>AgendarX v{version.replace(/-([a-z])$/, "$1")}</span>
      <span aria-hidden="true">·</span>
      <a
        href="https://www.fabianocesar.com"
        target="_blank"
        rel="noopener noreferrer"
        className="rounded-sm underline-offset-4 hover:text-teal-700 hover:underline focus-visible:outline-2 focus-visible:outline-teal-700"
      >
        www.fabianocesar.com
      </a>
    </footer>
  );
}
