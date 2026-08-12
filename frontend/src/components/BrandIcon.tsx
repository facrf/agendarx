import { useEffect, useState } from "react";
import { apiUrl } from "../services/api";
import { cn } from "./ui";

const BRANDING_EVENT = "agendarx:branding-updated";
const ICON_PATH = "/api/identidade/icone";

export function refreshBranding() {
  window.dispatchEvent(new Event(BRANDING_EVENT));
}

export function BrandIcon({ className }: { className?: string }) {
  const [revision, setRevision] = useState(() => Date.now());
  const [failed, setFailed] = useState(false);
  const iconUrl = apiUrl(`${ICON_PATH}?v=${revision}`);

  useEffect(() => {
    const refresh = () => {
      setFailed(false);
      setRevision(Date.now());
    };
    window.addEventListener(BRANDING_EVENT, refresh);
    return () => window.removeEventListener(BRANDING_EVENT, refresh);
  }, []);

  useEffect(() => {
    let favicon = document.querySelector<HTMLLinkElement>('link[rel="icon"]');
    if (!favicon) {
      favicon = document.createElement("link");
      favicon.rel = "icon";
      document.head.appendChild(favicon);
    }
    favicon.href = iconUrl;
  }, [iconUrl]);

  return (
    <div className={cn("grid shrink-0 place-items-center overflow-hidden rounded-2xl bg-coral font-bold text-white shadow-lg shadow-coral/20", className)}>
      {failed ? (
        <span aria-hidden="true">A</span>
      ) : (
        <img
          className="h-full w-full object-cover"
          src={iconUrl}
          alt=""
          onError={() => setFailed(true)}
        />
      )}
    </div>
  );
}
