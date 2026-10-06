/* Developed with care by FACRF - https://github.com/facrf */
import { Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
export function GlobalSearch() {
  const [query, setQuery] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") { event.preventDefault(); input.current?.focus(); }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  return <form className="mb-6 ml-auto flex w-full max-w-xl gap-2" role="search" onSubmit={event => {
    event.preventDefault(); if (query.trim()) navigate(`/busca?busca=${encodeURIComponent(query.trim())}`);
  }}>
    <label className="relative flex-1"><Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-slate-400" />
      <input ref={input} className="field pl-9" type="search" maxLength={200} aria-label="Buscar no sistema" placeholder="Buscar pessoas, tarefas, vínculos e arquivos…" value={query} onChange={event => setQuery(event.target.value)} />
    </label><button type="submit" className="btn btn-secondary" disabled={!query.trim()}>Buscar</button>
  </form>;
}
