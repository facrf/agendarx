import { ArrowRight, GitFork, Tag, X } from "lucide-react";
import { useEffect } from "react";
import type { GrafoEdge, GrafoNode } from "../types/api";

interface RelationshipDrawerProps {
  edge: GrafoEdge | null;
  nodes: GrafoNode[];
  onClose: () => void;
}

export function RelationshipDrawer({ edge, nodes, onClose }: RelationshipDrawerProps) {
  useEffect(() => {
    if (!edge) return;
    const closeOnEscape = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [edge, onClose]);

  if (!edge) return null;
  const source = nodes.find((node) => node.id === edge.source);
  const target = nodes.find((node) => node.id === edge.target);

  return (
    <div className="fixed inset-0 z-50" role="dialog" aria-modal="true" aria-label="Detalhes do vínculo">
      <button type="button" className="absolute inset-0 bg-slate-950/40 backdrop-blur-[2px]" onClick={onClose} aria-label="Fechar detalhes" />
      <aside className="animate-drawer absolute inset-y-0 right-0 flex w-full max-w-md flex-col bg-white shadow-2xl">
        <header className="flex items-center justify-between border-b border-slate-100 px-6 py-5">
          <div className="flex items-center gap-3">
            <div className="grid size-10 place-items-center rounded-2xl bg-coral/10 text-coral"><GitFork className="size-5" /></div>
            <div><p className="text-xs font-bold uppercase tracking-[0.16em] text-teal-700">Relação</p><h2 className="font-display text-xl font-semibold text-slate-950">{edge.label}</h2></div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="Fechar"><X className="size-5" /></button>
        </header>

        <div className="flex-1 overflow-y-auto p-6">
          <div className="flex items-center gap-3 rounded-2xl border border-teal-100 bg-teal-50/70 p-4">
            <PersonBadge node={source} />
            <ArrowRight className="size-5 shrink-0 text-coral" />
            <PersonBadge node={target} />
          </div>

          <div className="mt-6">
            <div className="mb-3 flex items-center gap-2"><Tag className="size-4 text-teal-700" /><h3 className="text-sm font-semibold text-slate-800">Histórico e descrição</h3></div>
            <div className="min-h-48 whitespace-pre-wrap rounded-2xl border border-slate-200 bg-slate-50 p-4 text-sm leading-7 text-slate-600">
              {edge.descricao || "Nenhuma descrição detalhada foi registrada para esta relação."}
            </div>
          </div>
        </div>
      </aside>
    </div>
  );
}

function PersonBadge({ node }: { node?: GrafoNode }) {
  const color = node?.color || "#86A6A3";
  return (
    <div className="min-w-0 flex-1 text-center">
      <div className="mx-auto grid size-11 place-items-center rounded-full border-[3px] bg-white text-sm font-bold uppercase" style={{ borderColor: color, color }}>
        {node?.label.slice(0, 2) || "?"}
      </div>
      <p className="mt-2 truncate text-sm font-semibold text-slate-900">{node?.label || "Pessoa removida"}</p>
      <p className="truncate text-[11px] text-slate-400">{node?.categoria || "Sem categoria"}</p>
    </div>
  );
}

