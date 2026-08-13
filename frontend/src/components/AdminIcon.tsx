import { useEffect, useState } from "react";
import { useAuth } from "../contexts/AuthContext";
import { apiUrl } from "../services/api";
import { cn } from "./ui";

export function AdminIcon({ className }: { className?: string }) {
  const { usuario } = useAuth();
  const [falhou, setFalhou] = useState(false);
  const revisao = usuario?.icone_atualizado_em || "padrao";

  useEffect(() => setFalhou(false), [revisao]);

  const iniciais = (usuario?.login || "A")
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((parte) => parte.charAt(0))
    .join("")
    .slice(0, 2)
    .toLocaleUpperCase("pt-BR");

  return (
    <div
      className={cn(
        "grid shrink-0 place-items-center overflow-hidden rounded-full bg-teal-700 font-semibold uppercase text-white",
        className,
      )}
    >
      {usuario?.tem_icone && !falhou ? (
        <img
          className="h-full w-full object-cover"
          src={apiUrl(`/api/auth/icone?v=${encodeURIComponent(revisao)}`)}
          alt={`Ícone de ${usuario.login}`}
          onError={() => setFalhou(true)}
        />
      ) : (
        <span aria-hidden="true">{iniciais}</span>
      )}
    </div>
  );
}
