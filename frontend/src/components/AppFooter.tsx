/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useState } from "react";
import { version as packageVersion } from "../../package.json";
import { api } from "../services/api";

let cachedSoftwareVersion: string | null = null;

export function AppFooter() {
  const [appVersion, setAppVersion] = useState<string>(
    cachedSoftwareVersion || packageVersion.replace(/-([a-z])$/, "$1"),
  );

  useEffect(() => {
    if (cachedSoftwareVersion) return;
    let active = true;
    api
      .get<{ versao?: string }>("/api/identidade/versao", { cacheMs: 60_000 })
      .then((data) => {
        if (active && data?.versao) {
          const v = data.versao.replace(/-([a-z])$/, "$1");
          cachedSoftwareVersion = v;
          setAppVersion(v);
        }
      })
      .catch(() => {
        // Mantém fallback do package.json
      });
    return () => {
      active = false;
    };
  }, []);

  return (
    <footer className="flex flex-wrap items-center justify-center gap-x-2 gap-y-1 px-4 py-5 text-center text-[11px] text-slate-500">
      <span>AgendarX v{appVersion}</span>
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
