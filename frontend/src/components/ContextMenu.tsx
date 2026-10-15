/* Developed with care by FACRF - https://github.com/facrf */
import { useEffect, useEffectEvent, useLayoutEffect, useRef } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";

export function ContextMenu({ x, y, title, items, onClose }: {
  x: number;
  y: number;
  title: string;
  items: Array<{ label: string; icon: ReactNode; action: () => void; danger?: boolean }>;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const close = useEffectEvent(onClose);
  const previousFocus = useRef<HTMLElement | null>(null);
  useLayoutEffect(() => {
    const menu = ref.current;
    if (!menu) return;
    const bounds = menu.getBoundingClientRect();
    menu.style.left = `${Math.max(8, Math.min(x, window.innerWidth - bounds.width - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(y, window.innerHeight - bounds.height - 8))}px`;
  }, [x, y, items.length]);
  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    previousFocus.current = previous;
    ref.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    const pointer = (event: PointerEvent) => { if (!ref.current?.contains(event.target as Node)) close(); };
    const dismiss = () => close();
    const keyboard = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); event.stopImmediatePropagation(); close(); }
      if (event.key === "Tab") { close(); return; }
      if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
        event.preventDefault();
        const buttons = [...(ref.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
        const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
        buttons[next]?.focus();
      }
    };
    window.addEventListener("pointerdown", pointer, true);
    window.addEventListener("keydown", keyboard, true);
    window.addEventListener("resize", dismiss);
    window.addEventListener("scroll", dismiss, true);
    window.addEventListener("wheel", dismiss, true);
    return () => {
      window.removeEventListener("pointerdown", pointer, true);
      window.removeEventListener("keydown", keyboard, true);
      window.removeEventListener("resize", dismiss);
      window.removeEventListener("scroll", dismiss, true);
      window.removeEventListener("wheel", dismiss, true);
      if (previous?.isConnected && !document.querySelector('[role="dialog"]')) previous.focus();
    };
  }, []);
  return createPortal(<div ref={ref} role="menu" aria-label={title} style={{ left: x, top: y }} className="fixed z-50 w-64 max-w-[calc(100vw-1rem)] rounded-2xl border border-slate-200 bg-white p-1.5 shadow-2xl" onContextMenu={event => event.preventDefault()}>
    <p className="truncate border-b border-slate-100 px-3 py-2 text-xs font-semibold text-slate-500" title={title}>{title}</p>
    {items.map(item => <button key={item.label} type="button" role="menuitem" tabIndex={-1} className={`flex min-h-11 w-full items-center gap-2 rounded-xl px-3 py-2 text-left text-sm focus-visible:outline-2 focus-visible:outline-teal-600 ${item.danger ? "text-rose-700 hover:bg-rose-50" : "text-slate-700 hover:bg-teal-50"}`} onClick={() => { previousFocus.current?.focus(); onClose(); item.action(); }}>{item.icon}{item.label}</button>)}
  </div>, document.body);
}
