/* Developed with care by FACRF - https://github.com/facrf */
import { Button } from "./ui";
export function Pagination({ page, pages, total, size, onPage, onSize, busy = false }: {
  page: number; pages: number; total: number; size: number;
  onPage: (page: number) => void; onSize?: (size: number) => void; busy?: boolean;
}) {
  return <nav className="mt-5 flex flex-wrap items-center justify-between gap-3 text-sm text-slate-600" aria-label="Paginação">
    <span role="status">{total} resultado(s) · página {page} de {Math.max(1, pages)}</span>
    <div className="flex items-center gap-2">
      {onSize && <select className="field w-auto" aria-label="Resultados por página" value={size} disabled={busy} onChange={e => onSize(Number(e.target.value))}>{[10, 30, 50, 100].map(n => <option key={n} value={n}>{n} por página</option>)}</select>}
      <Button type="button" variant="secondary" disabled={busy || page <= 1} onClick={() => onPage(page - 1)}>Anterior</Button>
      <Button type="button" variant="secondary" disabled={busy || page >= pages} onClick={() => onPage(page + 1)}>Próxima</Button>
    </div>
  </nav>;
}
