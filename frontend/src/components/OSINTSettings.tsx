/* Developed with care by FACRF - https://github.com/facrf */
import {
  AlertTriangle,
  CheckCircle2,
  KeyRound,
  Radar,
  Save,
  Trash2,
} from "lucide-react";
import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import { useToast } from "../contexts/ToastContext";
import { api, errorMessage } from "../services/api";
import type { FontePesquisaPublica, OsintConfiguracao } from "../types/api";
import { Button, Spinner, cn } from "./ui";

export function OSINTSettings() {
  const [config, setConfig] = useState<OsintConfiguracao | null>(null);
  const [carregando, setCarregando] = useState(true);
  const [salvando, setSalvando] = useState(false);

  const [cguApiKey, setCguApiKey] = useState("");
  const [datajudApiKey, setDatajudApiKey] = useState("");
  const [openalexApiKey, setOpenalexApiKey] = useState("");
  const [inlabsPassword, setInlabsPassword] = useState("");

  const { notify } = useToast();

  useEffect(() => {
    api
      .get<OsintConfiguracao>("/api/configuracoes/osint")
      .then(setConfig)
      .catch((error) => notify(errorMessage(error), "erro"))
      .finally(() => setCarregando(false));
  }, [notify]);

  const alternarFonte = (provider: FontePesquisaPublica) => {
    if (!config) return;
    const ativa = config.fontes_ativas.includes(provider);
    if (ativa && config.fontes_ativas.length <= 1) {
      notify("Mantenha ao menos um sistema de busca ativo", "erro");
      return;
    }
    const novas = ativa
      ? config.fontes_ativas.filter((f) => f !== provider)
      : [...config.fontes_ativas, provider];
    setConfig({ ...config, fontes_ativas: novas });
  };

  const salvar = async (event: FormEvent) => {
    event.preventDefault();
    if (!config) return;
    setSalvando(true);
    try {
      const atualizado = await api.put<OsintConfiguracao>("/api/configuracoes/osint", {
        searxng_url: config.searxng_url,
        cgu_api_key: cguApiKey.trim() || undefined,
        datajud_api_key: datajudApiKey.trim() || undefined,
        openalex_api_key: openalexApiKey.trim() || undefined,
        inlabs_username: config.inlabs_username,
        inlabs_password: inlabsPassword.trim() || undefined,
        inlabs_lookback_days: config.inlabs_lookback_days,
        pncp_base_url: config.pncp_base_url,
        pncp_tipos_documento: config.pncp_tipos_documento,
        pncp_retroativo_dias: config.pncp_retroativo_dias,
        cgu_base_url: config.cgu_base_url,
        tse_base_url: config.tse_base_url,
        tse_anos: config.tse_anos,
        tse_abrangencias: config.tse_abrangencias,
        tse_cargos: config.tse_cargos,
        osint_timeout_seconds: config.osint_timeout_seconds,
        osint_max_results: config.osint_max_results,
        fontes_ativas: config.fontes_ativas,
      });
      setConfig(atualizado);
      setCguApiKey("");
      setDatajudApiKey("");
      setOpenalexApiKey("");
      setInlabsPassword("");
      notify("Configurações de Pesquisa Pública (OSINT) salvas");
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  const limparSegredo = async (
    campo:
      | "limpar_cgu_api_key"
      | "limpar_datajud_api_key"
      | "limpar_openalex_api_key"
      | "limpar_inlabs_credenciais",
    rotulo: string,
  ) => {
    if (!window.confirm(`Remover ${rotulo} salva?`)) return;
    setSalvando(true);
    try {
      const atualizado = await api.put<OsintConfiguracao>("/api/configuracoes/osint", {
        [campo]: true,
      });
      setConfig(atualizado);
      notify(`${rotulo} removida`);
    } catch (error) {
      notify(errorMessage(error), "erro");
    } finally {
      setSalvando(false);
    }
  };

  return (
    <section className="panel overflow-hidden xl:col-span-2">
      <header className="flex items-center justify-between gap-3 border-b border-slate-100 p-5 sm:p-6">
        <div className="flex items-center gap-3">
          <div className="grid size-11 place-items-center rounded-2xl bg-teal-50 text-teal-700">
            <Radar className="size-5" />
          </div>
          <div>
            <h2 className="font-display text-xl font-semibold">
              Pesquisa Pública (OSINT): PNCP, CGU, TSE e Fontes Compatíveis
            </h2>
            <p className="text-sm text-slate-500">
              Configure chaves de API, abrangências eleitorais, janela de contratos e quais sistemas participam das buscas compatíveis.
            </p>
          </div>
        </div>
      </header>

      {carregando || !config ? (
        <div className="p-6">
          <Spinner label="Carregando configurações OSINT" />
        </div>
      ) : (
        <form className="space-y-6 p-5 sm:p-6" onSubmit={salvar}>
          <div>
            <h3 className="text-sm font-semibold text-slate-800">
              Sistemas habilitados no reaproveitamento automático de parâmetros
            </h3>
            <p className="mt-0.5 text-xs text-slate-500">
              Selecione quais bases públicas devem ser consultadas quando um parâmetro (ex.: Nome, CPF, CNPJ) usar a busca em sistemas compatíveis.
            </p>
            <div className="mt-3 grid gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
              {config.fontes_status.map((item) => {
                const ativo = config.fontes_ativas.includes(item.provider);
                return (
                  <label
                    key={item.provider}
                    className={cn(
                      "flex cursor-pointer flex-col justify-between rounded-2xl border p-3 text-xs transition",
                      ativo
                        ? "border-teal-200 bg-teal-50/40"
                        : "border-slate-200 bg-slate-50/60 opacity-75",
                    )}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <span className="font-semibold text-slate-900">{item.label}</span>
                      <input
                        type="checkbox"
                        className="mt-0.5 size-4 rounded border-slate-300 text-teal-600"
                        checked={ativo}
                        onChange={() => alternarFonte(item.provider)}
                      />
                    </div>
                    <div className="mt-2 flex flex-wrap gap-1">
                      {item.tipos_compativeis.map((t) => (
                        <span
                          key={t}
                          className="rounded bg-white px-1.5 py-0.5 text-[0.6rem] font-semibold text-slate-600 shadow-2xs"
                        >
                          {t}
                        </span>
                      ))}
                    </div>
                    <div className="mt-2 flex items-center gap-1 text-[0.65rem] font-medium">
                      {item.configurado ? (
                        <span className="inline-flex items-center gap-1 text-emerald-700">
                          <CheckCircle2 className="size-3.5" /> Pronto para uso
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1 text-amber-700">
                          <AlertTriangle className="size-3.5" /> Aguarda credencial/URL abaixo
                        </span>
                      )}
                    </div>
                  </label>
                );
              })}
            </div>
          </div>

          <div className="grid gap-5 lg:grid-cols-3">
            {/* Bloco 1: CGU */}
            <div className="rounded-2xl border border-slate-200 bg-slate-50/60 p-4 space-y-3">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-sm font-semibold text-slate-900">
                  APIs da CGU (Portal da Transparência)
                </h3>
                <span
                  className={cn(
                    "rounded-full px-2 py-0.5 text-[0.65rem] font-semibold",
                    config.cgu_api_key_configurada
                      ? "bg-emerald-100 text-emerald-800"
                      : "bg-amber-100 text-amber-800",
                  )}
                >
                  {config.cgu_api_key_configurada ? "Chave ativa" : "Requer chave"}
                </span>
              </div>
              <p className="text-xs leading-5 text-slate-500">
                Consulta CEIS, CNEP, CEPIM, contratos federais e servidores por CPF, CNPJ ou Nome. Solicite gratuitamente o token em{" "}
                <a
                  href="https://portaldatransparencia.gov.br/api-de-dados/cadastrar-email"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="font-semibold text-teal-700 underline"
                >
                  portaldatransparencia.gov.br
                </a>
                .
              </p>
              <div>
                <label className="field-label" htmlFor="cgu-api-key">
                  Chave da API de Dados (chave-api-dados)
                </label>
                <div className="flex gap-2">
                  <input
                    id="cgu-api-key"
                    type="password"
                    className="field flex-1"
                    value={cguApiKey}
                    placeholder={
                      config.cgu_api_key_mascarada
                        ? `Salva (${config.cgu_api_key_mascarada}) — digite para alterar`
                        : "Cole seu token chave-api-dados"
                    }
                    onChange={(e) => setCguApiKey(e.target.value)}
                  />
                  {config.cgu_api_key_configurada && (
                    <Button
                      type="button"
                      variant="ghost"
                      title="Remover chave da CGU"
                      onClick={() => void limparSegredo("limpar_cgu_api_key", "a chave da CGU")}
                    >
                      <Trash2 className="size-4 text-rose-600" />
                    </Button>
                  )}
                </div>
              </div>
              <div>
                <label className="field-label" htmlFor="cgu-base-url">
                  URL Base da API da CGU
                </label>
                <input
                  id="cgu-base-url"
                  className="field"
                  value={config.cgu_base_url}
                  onChange={(e) => setConfig({ ...config, cgu_base_url: e.target.value })}
                />
              </div>
            </div>

            {/* Bloco 2: PNCP */}
            <div className="rounded-2xl border border-slate-200 bg-slate-50/60 p-4 space-y-3">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-sm font-semibold text-slate-900">
                  PNCP (Contratações Públicas)
                </h3>
                <span className="rounded-full bg-emerald-100 px-2 py-0.5 text-[0.65rem] font-semibold text-emerald-800">
                  API Pública
                </span>
              </div>
              <p className="text-xs leading-5 text-slate-500">
                Pesquisa editais, atas e contratos públicos por CNPJ de fornecedor/órgão, nome, processo ou termo livre sem exigir chave.
              </p>
              <div>
                <label className="field-label" htmlFor="pncp-tipos-doc">
                  Tipos de documento consultados
                </label>
                <input
                  id="pncp-tipos-doc"
                  className="field"
                  value={config.pncp_tipos_documento}
                  placeholder="edital|ata|contrato"
                  onChange={(e) =>
                    setConfig({ ...config, pncp_tipos_documento: e.target.value })
                  }
                />
              </div>
              <div>
                <label className="field-label" htmlFor="pncp-dias">
                  Janela retroativa de contratos por CNPJ (dias)
                </label>
                <input
                  id="pncp-dias"
                  type="number"
                  min={1}
                  max={3650}
                  className="field"
                  value={config.pncp_retroativo_dias}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      pncp_retroativo_dias: Number(e.target.value) || 365,
                    })
                  }
                />
              </div>
              <div>
                <label className="field-label" htmlFor="pncp-base-url">
                  URL Base do PNCP
                </label>
                <input
                  id="pncp-base-url"
                  className="field"
                  value={config.pncp_base_url}
                  onChange={(e) => setConfig({ ...config, pncp_base_url: e.target.value })}
                />
              </div>
            </div>

            {/* Bloco 3: TSE DivulgaCandContas */}
            <div className="rounded-2xl border border-slate-200 bg-slate-50/60 p-4 space-y-3">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-sm font-semibold text-slate-900">
                  DivulgaCandContas (TSE)
                </h3>
                <span className="rounded-full bg-emerald-100 px-2 py-0.5 text-[0.65rem] font-semibold text-emerald-800">
                  API Pública
                </span>
              </div>
              <p className="text-xs leading-5 text-slate-500">
                Consulta candidaturas, bens declarados e CNPJ de campanha por Nome, CPF ou CNPJ nos anos e UFs/municípios configurados.
              </p>
              <div>
                <label className="field-label" htmlFor="tse-anos">
                  Anos eleitorais (separados por vírgula)
                </label>
                <input
                  id="tse-anos"
                  className="field"
                  value={config.tse_anos}
                  placeholder="2024, 2022, 2020, 2018"
                  onChange={(e) => setConfig({ ...config, tse_anos: e.target.value })}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="tse-abrangencias">
                  UFs / Códigos TSE de Município (UE)
                </label>
                <input
                  id="tse-abrangencias"
                  className="field"
                  value={config.tse_abrangencias}
                  placeholder="BR, SP, RJ, MG, DF"
                  onChange={(e) => setConfig({ ...config, tse_abrangencias: e.target.value })}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="tse-cargos">
                  Códigos de cargos TSE (1=Pres, 3=Gov, 5=Sen, 6=DepFed, 11=Pref, 13=Ver)
                </label>
                <input
                  id="tse-cargos"
                  className="field"
                  value={config.tse_cargos}
                  placeholder="1,3,5,6,7,11,13"
                  onChange={(e) => setConfig({ ...config, tse_cargos: e.target.value })}
                />
              </div>
            </div>
          </div>

          <details className="rounded-2xl border border-slate-200 bg-slate-50/40 p-4">
            <summary className="flex cursor-pointer items-center gap-2 text-sm font-semibold text-slate-800">
              <KeyRound className="size-4 text-teal-700" />
              Outras integrações e limites avançados (SearXNG, INLABS/DOU, DataJud, OpenAlex)
            </summary>
            <div className="mt-4 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              <div>
                <label className="field-label" htmlFor="osint-searxng-url">
                  URL da instância SearXNG
                </label>
                <input
                  id="osint-searxng-url"
                  className="field"
                  value={config.searxng_url}
                  placeholder="http://searxng:8080"
                  onChange={(e) => setConfig({ ...config, searxng_url: e.target.value })}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="osint-datajud-key">
                  Chave DataJud / CNJ (opcional — possui padrão público)
                </label>
                <input
                  id="osint-datajud-key"
                  type="password"
                  className="field"
                  value={datajudApiKey}
                  placeholder={
                    config.datajud_api_key_mascarada
                      ? `Salva (${config.datajud_api_key_mascarada})`
                      : "Usa chave pública oficial do CNJ"
                  }
                  onChange={(e) => setDatajudApiKey(e.target.value)}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="osint-openalex-key">
                  Chave OpenAlex (opcional)
                </label>
                <input
                  id="osint-openalex-key"
                  type="password"
                  className="field"
                  value={openalexApiKey}
                  placeholder={
                    config.openalex_api_key_mascarada
                      ? `Salva (${config.openalex_api_key_mascarada})`
                      : "Opcional"
                  }
                  onChange={(e) => setOpenalexApiKey(e.target.value)}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="osint-inlabs-user">
                  Usuário INLABS / Imprensa Nacional (e-mail)
                </label>
                <input
                  id="osint-inlabs-user"
                  className="field"
                  value={config.inlabs_username}
                  placeholder="email@exemplo.gov.br"
                  onChange={(e) => setConfig({ ...config, inlabs_username: e.target.value })}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="osint-inlabs-pass">
                  Senha INLABS
                </label>
                <input
                  id="osint-inlabs-pass"
                  type="password"
                  className="field"
                  value={inlabsPassword}
                  placeholder={
                    config.inlabs_configurado
                      ? "Senha salva — digite para alterar"
                      : "Senha de acesso ao INLABS"
                  }
                  onChange={(e) => setInlabsPassword(e.target.value)}
                />
              </div>
              <div>
                <label className="field-label" htmlFor="osint-max-results">
                  Máximo de resultados por fonte (1 a 100)
                </label>
                <input
                  id="osint-max-results"
                  type="number"
                  min={1}
                  max={100}
                  className="field"
                  value={config.osint_max_results}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      osint_max_results: Number(e.target.value) || 15,
                    })
                  }
                />
              </div>
            </div>
          </details>

          <div className="flex justify-end">
            <Button type="submit" loading={salvando}>
              <Save className="size-4" /> Salvar configurações de pesquisa pública
            </Button>
          </div>
        </form>
      )}
    </section>
  );
}
