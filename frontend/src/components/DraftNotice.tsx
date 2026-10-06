/* Developed with care by FACRF - https://github.com/facrf */
import { Button } from "./ui";
export function DraftNotice({ draft }: { draft: { pending: { updated: number } | null; savedAt: number | null; unavailable: boolean; recover: () => void; clear: () => void } }) {
  if (draft.pending) return <div className="my-3 flex flex-wrap items-center gap-3 rounded-xl border border-amber-200 bg-amber-50 p-3 text-sm" role="status">
    <span>Rascunho de {new Date(draft.pending.updated).toLocaleString("pt-BR")} disponível.</span>
    <Button type="button" variant="secondary" onClick={draft.recover}>Recuperar rascunho</Button>
    <Button type="button" variant="ghost" onClick={draft.clear}>Descartar rascunho</Button>
  </div>;
  if (draft.unavailable) return <p className="my-2 text-xs text-amber-700" role="status">O navegador não conseguiu salvar o rascunho. Salve o formulário antes de sair.</p>;
  return <p className="my-2 text-xs text-slate-500" role="status">{draft.savedAt ? "Rascunho salvo neste navegador." : "Alterações serão salvas como rascunho neste navegador."} Fotos e arquivos precisam ser selecionados novamente.</p>;
}
