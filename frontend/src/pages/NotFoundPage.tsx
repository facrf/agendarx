import { ArrowLeft, Compass } from "lucide-react";
import { Link } from "react-router-dom";

export function NotFoundPage() {
  return (
    <div className="grid min-h-[70vh] place-items-center text-center">
      <div>
        <Compass className="mx-auto size-12 text-teal-700" />
        <p className="eyebrow mt-5">Erro 404</p>
        <h1 className="font-display text-4xl font-semibold text-slate-950">Esta página não está no mapa</h1>
        <p className="mt-3 text-slate-500">O endereço pode ter mudado ou não existir.</p>
        <Link className="btn btn-primary mt-6" to="/pessoas">
          <ArrowLeft className="size-4" /> Voltar para pessoas
        </Link>
      </div>
    </div>
  );
}

