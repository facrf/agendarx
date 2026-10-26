/* Developed with care by FACRF - https://github.com/facrf */
import {
  AlignCenter,
  AlignJustify,
  AlignLeft,
  AlignRight,
  Bold,
  Code,
  Columns,
  Eye,
  FileCode,
  FilePenLine,
  FileText,
  Heading1,
  Heading2,
  Heading3,
  Italic,
  Link as LinkIcon,
  List,
  ListOrdered,
  Maximize2,
  Minimize2,
  Minus,
  Quote,
  Redo2,
  RemoveFormatting,
  Save,
  Strikethrough,
  Table as TableIcon,
  Trash2,
  Type,
  Underline,
  Undo2,
  X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, errorMessage } from "../services/api";
import type { AnexoDossie, DocumentoDossie } from "../types/api";
import { useToast } from "../contexts/ToastContext";
import { htmlToMarkdown, markdownToHtml } from "../utils/documentConverter";
import { MarkdownText } from "./MarkdownText";
import { Button, Spinner, cn } from "./ui";

interface DocumentEditorModalProps {
  open: boolean;
  pessoaId: number;
  documentoId?: number | null;
  initialFormat?: "markdown" | "html";
  onClose: () => void;
  onSaved: (anexo: AnexoDossie) => void;
  onDeleted?: (id: number) => void;
}

type ViewMode = "visual" | "codigo" | "split" | "preview";

export function DocumentEditorModal({
  open,
  pessoaId,
  documentoId,
  initialFormat = "markdown",
  onClose,
  onSaved,
  onDeleted,
}: DocumentEditorModalProps) {
  const [nome, setNome] = useState("");
  const [formato, setFormato] = useState<"markdown" | "html">(initialFormat);
  const [conteudo, setConteudo] = useState("");
  const [viewMode, setViewMode] = useState<ViewMode>("visual");
  const [carregando, setCarregando] = useState(false);
  const [salvando, setSalvando] = useState(false);
  const [excluindo, setExcluindo] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [modificado, setModificado] = useState(false);

  const visualRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const { notify } = useToast();

  // Carregar dados quando editando documento existente
  useEffect(() => {
    if (!open) return;
    if (documentoId) {
      setCarregando(true);
      setModificado(false);
      api
        .get<DocumentoDossie>(`/api/dossie/anexos/${documentoId}/documento`)
        .then((doc) => {
          let baseNome = doc.nome_arquivo;
          const isHtml =
            doc.mime_type.includes("html") ||
            /\.html?$/i.test(doc.nome_arquivo);
          const isMd =
            doc.mime_type.includes("markdown") ||
            /\.md$/i.test(doc.nome_arquivo);

          const fmt = isHtml ? "html" : isMd ? "markdown" : "markdown";
          setFormato(fmt);
          baseNome = baseNome.replace(/\.(html?|markdown|md|txt)$/i, "");
          setNome(baseNome);
          setConteudo(doc.conteudo);
          setModificado(false);
        })
        .catch((err) => {
          notify(errorMessage(err), "erro");
          onClose();
        })
        .finally(() => {
          setCarregando(false);
        });
    } else {
      // Novo documento
      setNome("Novo documento");
      setFormato(initialFormat);
      setConteudo(
        initialFormat === "html"
          ? "<h1>Título do Documento</h1><p>Digite seu texto aqui...</p>"
          : "# Título do Documento\n\nDigite seu texto aqui...",
      );
      setViewMode("visual");
      setModificado(false);
      setCarregando(false);
    }
  }, [open, documentoId, initialFormat, onClose, notify]);

  // Sincronizar o DOM do contentEditable quando mudar o conteúdo externamente
  useEffect(() => {
    if (visualRef.current && viewMode === "visual") {
      const htmlToRender =
        formato === "html" ? conteudo : markdownToHtml(conteudo);
      if (visualRef.current.innerHTML !== htmlToRender) {
        visualRef.current.innerHTML = htmlToRender;
      }
    }
  }, [viewMode, formato, conteudo]);

  // Estatísticas de palavras e caracteres
  const estatisticas = useMemo(() => {
    const textoPuro =
      formato === "html"
        ? conteudo.replace(/<[^>]+>/g, " ")
        : conteudo.replace(/[#*`~_>[\]()]/g, " ");
    const palavras = textoPuro
      .trim()
      .split(/\s+/)
      .filter((w) => w.length > 0).length;
    const caracteres = textoPuro.length;
    return { palavras, caracteres };
  }, [conteudo, formato]);

  // Inserir sintaxe no editor Markdown
  const inserirNoMarkdown = useCallback((comando: string, extra?: string) => {
    const el = textareaRef.current;
    if (!el) return;

    const start = el.selectionStart;
    const end = el.selectionEnd;
    const text = el.value;
    const selected = text.slice(start, end);

    let prefix = "";
    let suffix = "";
    let replacement: string | null = null;

    switch (comando) {
      case "bold":
        prefix = "**";
        suffix = "**";
        break;
      case "italic":
        prefix = "*";
        suffix = "*";
        break;
      case "underline":
        prefix = "<u>";
        suffix = "</u>";
        break;
      case "strikeThrough":
        prefix = "~~";
        suffix = "~~";
        break;
      case "h1":
        prefix = "\n# ";
        break;
      case "h2":
        prefix = "\n## ";
        break;
      case "h3":
        prefix = "\n### ";
        break;
      case "quote":
        prefix = "\n> ";
        break;
      case "code":
        if (selected.includes("\n")) {
          prefix = "\n```\n";
          suffix = "\n```\n";
        } else {
          prefix = "`";
          suffix = "`";
        }
        break;
      case "ul":
        prefix = "\n- ";
        break;
      case "ol":
        prefix = "\n1. ";
        break;
      case "hr":
        replacement = "\n\n---\n\n";
        break;
      case "link": {
        const url = extra || prompt("Insira a URL do link:", "https://");
        if (!url) return;
        replacement = `[${selected || "texto do link"}](${url})`;
        break;
      }
      case "table":
        replacement =
          "\n\n| Coluna 1 | Coluna 2 | Coluna 3 |\n| --- | --- | --- |\n| Item 1 | Item 2 | Item 3 |\n| Item 4 | Item 5 | Item 6 |\n\n";
        break;
      default:
        return;
    }

    const newText =
      replacement !== null
        ? text.slice(0, start) + replacement + text.slice(end)
        : text.slice(0, start) + prefix + (selected || "texto") + suffix + text.slice(end);

    setConteudo(newText);
    setModificado(true);

    setTimeout(() => {
      el.focus();
      const pos =
        replacement !== null
          ? start + replacement.length
          : start + prefix.length + (selected ? selected.length : 5);
      el.setSelectionRange(pos, pos);
    }, 10);
  }, []);

  // Manipular comandos do editor visual (estilo Word)
  const executarComando = useCallback(
    (comando: string, valor: string | undefined = undefined) => {
      if (viewMode === "visual" && visualRef.current) {
        visualRef.current.focus();
        document.execCommand(comando, false, valor);
        const novoHtml = visualRef.current.innerHTML;
        if (formato === "html") {
          setConteudo(novoHtml);
        } else {
          setConteudo(htmlToMarkdown(novoHtml));
        }
        setModificado(true);
      } else if (textareaRef.current) {
        // Manipular inserção no textarea
        inserirNoMarkdown(comando, valor);
      }
    },
    [viewMode, formato, inserirNoMarkdown],
  );

  // Inserir link
  const inserirLink = () => {
    const url = prompt("Digite a URL do link:", "https://");
    if (!url) return;
    if (viewMode === "visual") {
      executarComando("createLink", url);
    } else {
      inserirNoMarkdown("link", url);
    }
  };

  // Inserir tabela estilo Word
  const inserirTabela = () => {
    if (viewMode === "visual") {
      const tabelaHtml = `
        <table style="width: 100%; border-collapse: collapse; margin: 1rem 0;">
          <thead>
            <tr>
              <th style="border: 1px solid #cbd5e1; padding: 8px; background: #f8fafc; font-weight: 600;">Coluna 1</th>
              <th style="border: 1px solid #cbd5e1; padding: 8px; background: #f8fafc; font-weight: 600;">Coluna 2</th>
              <th style="border: 1px solid #cbd5e1; padding: 8px; background: #f8fafc; font-weight: 600;">Coluna 3</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td style="border: 1px solid #cbd5e1; padding: 8px;">Item 1</td>
              <td style="border: 1px solid #cbd5e1; padding: 8px;">Item 2</td>
              <td style="border: 1px solid #cbd5e1; padding: 8px;">Item 3</td>
            </tr>
            <tr>
              <td style="border: 1px solid #cbd5e1; padding: 8px;">Item 4</td>
              <td style="border: 1px solid #cbd5e1; padding: 8px;">Item 5</td>
              <td style="border: 1px solid #cbd5e1; padding: 8px;">Item 6</td>
            </tr>
          </tbody>
        </table>
        <p><br></p>
      `;
      executarComando("insertHTML", tabelaHtml);
    } else {
      inserirNoMarkdown("table");
    }
  };

  // Alternar formato (Markdown <-> HTML)
  const mudarFormato = (novoFormato: "markdown" | "html") => {
    if (novoFormato === formato) return;
    if (novoFormato === "html") {
      setConteudo(markdownToHtml(conteudo));
    } else {
      setConteudo(htmlToMarkdown(conteudo));
    }
    setFormato(novoFormato);
    setModificado(true);
  };

  // Salvar documento
  const salvar = useCallback(async () => {
    const nomeLimpo = nome.trim();
    if (!nomeLimpo) {
      notify("Informe o nome do documento", "erro");
      return;
    }

    setSalvando(true);
    try {
      const extensao = formato === "html" ? ".html" : ".md";
      const nomeCompleto = nomeLimpo.toLowerCase().endsWith(extensao)
        ? nomeLimpo
        : `${nomeLimpo}${extensao}`;

      let res: AnexoDossie;
      if (documentoId) {
        res = await api.put<AnexoDossie>(
          `/api/dossie/anexos/${documentoId}/documento`,
          {
            nome_arquivo: nomeCompleto,
            conteudo,
            formato,
          },
        );
        notify("Documento atualizado com sucesso");
      } else {
        res = await api.post<AnexoDossie>(
          `/api/dossie/pessoas/${pessoaId}/documentos`,
          {
            nome_arquivo: nomeCompleto,
            conteudo,
            formato,
          },
        );
        notify("Documento criado com sucesso no dossiê");
      }

      setModificado(false);
      onSaved(res);
      onClose();
    } catch (err) {
      notify(errorMessage(err), "erro");
    } finally {
      setSalvando(false);
    }
  }, [nome, formato, conteudo, documentoId, pessoaId, notify, onSaved, onClose]);

  // Excluir documento
  const excluir = async () => {
    if (!documentoId) return;
    if (!window.confirm(`Deseja realmente excluir o documento “${nome}” do dossiê?`)) {
      return;
    }

    setExcluindo(true);
    try {
      await api.delete(`/api/dossie/anexos/${documentoId}`);
      notify("Documento excluído do dossiê");
      onDeleted?.(documentoId);
      onClose();
    } catch (err) {
      notify(errorMessage(err), "erro");
    } finally {
      setExcluindo(false);
    }
  };

  // Fechar com aviso de alterações pendentes
  const fechar = () => {
    if (
      modificado &&
      !window.confirm("Há alterações não salvas. Deseja fechar e descartar?")
    ) {
      return;
    }
    onClose();
  };

  // Atalhos de teclado (Ctrl+S, Ctrl+B, etc.)
  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void salvar();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, salvar]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/60 p-2 backdrop-blur-xs sm:p-4"
      role="dialog"
      aria-modal="true"
    >
      <div
        className={cn(
          "flex flex-col overflow-hidden rounded-3xl bg-white shadow-2xl transition-all duration-200 border border-slate-200",
          maximized
            ? "h-full w-full rounded-none"
            : "h-[min(94dvh,920px)] w-[min(96vw,1200px)]",
        )}
      >
        {/* Cabeçalho do Editor */}
        <header className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-200 bg-white px-4 py-3 sm:px-6">
          <div className="flex min-w-0 flex-1 items-center gap-3">
            <div className="grid size-10 shrink-0 place-items-center rounded-xl bg-teal-100 text-teal-800">
              {formato === "html" ? (
                <FileCode className="size-5" />
              ) : (
                <FileText className="size-5" />
              )}
            </div>
            <div className="min-w-0 flex-1">
              <input
                type="text"
                value={nome}
                onChange={(e) => {
                  setNome(e.target.value);
                  setModificado(true);
                }}
                placeholder="Nome do documento..."
                className="w-full truncate border-none bg-transparent p-0 text-base font-bold text-slate-900 outline-none placeholder:text-slate-400 focus:ring-0 sm:text-lg"
              />
              <div className="flex items-center gap-2 text-xs text-slate-500">
                <span className="font-medium text-slate-700">
                  {documentoId ? "Editando documento" : "Novo documento"}
                </span>
                <span>•</span>
                <span
                  className={cn(
                    "font-semibold",
                    modificado ? "text-amber-600" : "text-emerald-600",
                  )}
                >
                  {modificado ? "Alterações não salvas" : "Salvo"}
                </span>
              </div>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {/* Seletor de Formato: HTML / Markdown */}
            <div className="inline-flex rounded-xl bg-slate-100 p-1 text-xs font-semibold text-slate-600">
              <button
                type="button"
                className={cn(
                  "rounded-lg px-2.5 py-1.5 transition",
                  formato === "markdown"
                    ? "bg-white text-teal-800 shadow-xs"
                    : "hover:text-slate-900",
                )}
                onClick={() => mudarFormato("markdown")}
                title="Formato Markdown (.md)"
              >
                Markdown (.md)
              </button>
              <button
                type="button"
                className={cn(
                  "rounded-lg px-2.5 py-1.5 transition",
                  formato === "html"
                    ? "bg-white text-teal-800 shadow-xs"
                    : "hover:text-slate-900",
                )}
                onClick={() => mudarFormato("html")}
                title="Formato HTML (.html)"
              >
                HTML (.html)
              </button>
            </div>

            {documentoId && (
              <Button
                type="button"
                variant="danger"
                loading={excluindo}
                disabled={salvando}
                onClick={() => void excluir()}
                title="Excluir documento"
              >
                <Trash2 className="size-4" />{" "}
                <span className="hidden sm:inline">Excluir</span>
              </Button>
            )}

            <Button
              type="button"
              loading={salvando}
              disabled={excluindo}
              onClick={() => void salvar()}
              title="Salvar documento (Ctrl+S)"
            >
              <Save className="size-4" /> Salvar
            </Button>

            <button
              type="button"
              className="rounded-xl p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-800"
              onClick={() => setMaximized(!maximized)}
              title={maximized ? "Restaurar tamanho" : "Maximizar"}
            >
              {maximized ? (
                <Minimize2 className="size-5" />
              ) : (
                <Maximize2 className="size-5" />
              )}
            </button>

            <button
              type="button"
              className="rounded-xl p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-800"
              onClick={fechar}
              title="Fechar"
            >
              <X className="size-5" />
            </button>
          </div>
        </header>

        {/* Barra de Ferramentas Estilo Word (Ribbon) */}
        <section
          className="flex flex-wrap items-center gap-1 border-b border-slate-200 bg-slate-50/90 px-3 py-1.5 text-slate-700 shadow-xs"
          aria-label="Barra de formatação de documento"
        >
          {/* Estilos de Cabeçalho */}
          <div className="flex items-center gap-0.5 border-r border-slate-200 pr-1.5 mr-1">
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("formatBlock", "<p>")
                  : inserirNoMarkdown("p")
              }
              title="Parágrafo normal"
            >
              <Type className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("formatBlock", "<h1>")
                  : inserirNoMarkdown("h1")
              }
              title="Título 1 (H1)"
            >
              <Heading1 className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("formatBlock", "<h2>")
                  : inserirNoMarkdown("h2")
              }
              title="Título 2 (H2)"
            >
              <Heading2 className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("formatBlock", "<h3>")
                  : inserirNoMarkdown("h3")
              }
              title="Título 3 (H3)"
            >
              <Heading3 className="size-4" />
            </button>
          </div>

          {/* Formatação Básica de Texto (Negrito, Itálico, etc.) */}
          <div className="flex items-center gap-0.5 border-r border-slate-200 pr-1.5 mr-1">
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200 font-bold"
              onClick={() => executarComando("bold")}
              title="Negrito (Ctrl+B)"
            >
              <Bold className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200 italic"
              onClick={() => executarComando("italic")}
              title="Itálico (Ctrl+I)"
            >
              <Italic className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200 underline"
              onClick={() => executarComando("underline")}
              title="Sublinhado (Ctrl+U)"
            >
              <Underline className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200 line-through"
              onClick={() => executarComando("strikeThrough")}
              title="Tachado"
            >
              <Strikethrough className="size-4" />
            </button>
          </div>

          {/* Alinhamento de Texto (para HTML e modo visual) */}
          <div className="flex items-center gap-0.5 border-r border-slate-200 pr-1.5 mr-1">
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("justifyLeft")}
              title="Alinhar à esquerda"
            >
              <AlignLeft className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("justifyCenter")}
              title="Centralizar"
            >
              <AlignCenter className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("justifyRight")}
              title="Alinhar à direita"
            >
              <AlignRight className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("justifyFull")}
              title="Justificar"
            >
              <AlignJustify className="size-4" />
            </button>
          </div>

          {/* Listas e Estrutura */}
          <div className="flex items-center gap-0.5 border-r border-slate-200 pr-1.5 mr-1">
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("insertUnorderedList")
                  : inserirNoMarkdown("ul")
              }
              title="Lista de marcadores"
            >
              <List className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("insertOrderedList")
                  : inserirNoMarkdown("ol")
              }
              title="Lista numerada"
            >
              <ListOrdered className="size-4" />
            </button>
          </div>

          {/* Inserções: Link, Tabela, Citação, Linha, Código */}
          <div className="flex items-center gap-0.5 border-r border-slate-200 pr-1.5 mr-1">
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={inserirLink}
              title="Inserir link"
            >
              <LinkIcon className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={inserirTabela}
              title="Inserir tabela estilo Word"
            >
              <TableIcon className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("formatBlock", "<blockquote>")
                  : inserirNoMarkdown("quote")
              }
              title="Citação"
            >
              <Quote className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("formatBlock", "<pre>")
                  : inserirNoMarkdown("code")
              }
              title="Bloco de código"
            >
              <Code className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() =>
                viewMode === "visual"
                  ? executarComando("insertHorizontalRule")
                  : inserirNoMarkdown("hr")
              }
              title="Linha divisória horizontal"
            >
              <Minus className="size-4" />
            </button>
          </div>

          {/* Desfazer / Refazer / Limpar */}
          <div className="flex items-center gap-0.5 border-r border-slate-200 pr-1.5 mr-1">
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("undo")}
              title="Desfazer (Ctrl+Z)"
            >
              <Undo2 className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("redo")}
              title="Refazer (Ctrl+Y)"
            >
              <Redo2 className="size-4" />
            </button>
            <button
              type="button"
              className="rounded-lg p-1.5 hover:bg-slate-200"
              onClick={() => executarComando("removeFormat")}
              title="Limpar formatação"
            >
              <RemoveFormatting className="size-4" />
            </button>
          </div>

          {/* Modos de Visualização */}
          <div className="ml-auto flex items-center rounded-lg bg-slate-200/80 p-0.5 text-xs font-medium">
            <button
              type="button"
              className={cn(
                "flex items-center gap-1 rounded-md px-2.5 py-1 transition",
                viewMode === "visual"
                  ? "bg-white text-teal-800 shadow-xs font-semibold"
                  : "text-slate-600 hover:text-slate-900",
              )}
              onClick={() => {
                setViewMode("visual");
              }}
              title="Editor visual estilo Word"
            >
              <FilePenLine className="size-3.5" /> Visual (Word)
            </button>

            <button
              type="button"
              className={cn(
                "flex items-center gap-1 rounded-md px-2.5 py-1 transition",
                viewMode === "codigo"
                  ? "bg-white text-teal-800 shadow-xs font-semibold"
                  : "text-slate-600 hover:text-slate-900",
              )}
              onClick={() => {
                if (viewMode === "visual" && visualRef.current) {
                  const html = visualRef.current.innerHTML;
                  setConteudo(
                    formato === "html" ? html : htmlToMarkdown(html),
                  );
                }
                setViewMode("codigo");
              }}
              title="Código fonte (HTML ou Markdown)"
            >
              <Code className="size-3.5" /> Código
            </button>

            {formato === "markdown" && (
              <button
                type="button"
                className={cn(
                  "hidden sm:flex items-center gap-1 rounded-md px-2.5 py-1 transition",
                  viewMode === "split"
                    ? "bg-white text-teal-800 shadow-xs font-semibold"
                    : "text-slate-600 hover:text-slate-900",
                )}
                onClick={() => {
                  if (viewMode === "visual" && visualRef.current) {
                    setConteudo(htmlToMarkdown(visualRef.current.innerHTML));
                  }
                  setViewMode("split");
                }}
                title="Editor e prévia lado a lado"
              >
                <Columns className="size-3.5" /> Dividido
              </button>
            )}

            <button
              type="button"
              className={cn(
                "flex items-center gap-1 rounded-md px-2.5 py-1 transition",
                viewMode === "preview"
                  ? "bg-white text-teal-800 shadow-xs font-semibold"
                  : "text-slate-600 hover:text-slate-900",
              )}
              onClick={() => {
                if (viewMode === "visual" && visualRef.current) {
                  const html = visualRef.current.innerHTML;
                  setConteudo(
                    formato === "html" ? html : htmlToMarkdown(html),
                  );
                }
                setViewMode("preview");
              }}
              title="Visualização final da página"
            >
              <Eye className="size-3.5" /> Prévia
            </button>
          </div>
        </section>

        {/* Área Central: Mesa de Trabalho com a Folha de Papel (Estilo Word) */}
        <main className="flex-1 overflow-y-auto bg-slate-100 p-4 sm:p-8">
          {carregando ? (
            <div className="flex h-64 items-center justify-center">
              <Spinner label="Carregando documento..." />
            </div>
          ) : viewMode === "visual" ? (
            /* Folha de Papel A4 Estilo Word (WYSIWYG) */
            <div className="mx-auto min-h-[620px] max-w-[850px] rounded-sm border border-slate-200/90 bg-white p-8 shadow-xl ring-1 ring-slate-900/5 sm:p-14">
              <div
                ref={visualRef}
                contentEditable
                suppressContentEditableWarning
                onInput={() => {
                  if (visualRef.current) {
                    const html = visualRef.current.innerHTML;
                    if (formato === "html") {
                      setConteudo(html);
                    } else {
                      setConteudo(htmlToMarkdown(html));
                    }
                    setModificado(true);
                  }
                }}
                className="min-h-[500px] outline-none text-slate-800 font-sans leading-relaxed
                  [&>h1]:text-3xl [&>h1]:font-bold [&>h1]:mb-4 [&>h1]:text-slate-900
                  [&>h2]:text-2xl [&>h2]:font-bold [&>h2]:mb-3 [&>h2]:text-slate-900
                  [&>h3]:text-xl [&>h3]:font-semibold [&>h3]:mb-2 [&>h3]:text-slate-900
                  [&>p]:mb-3.5 [&>p]:leading-7
                  [&>ul]:list-disc [&>ul]:pl-6 [&>ul]:mb-3.5
                  [&>ol]:list-decimal [&>ol]:pl-6 [&>ol]:mb-3.5
                  [&>blockquote]:border-l-4 [&>blockquote]:border-teal-600 [&>blockquote]:pl-4 [&>blockquote]:italic [&>blockquote]:text-slate-600 [&>blockquote]:my-4
                  [&>hr]:my-6 [&>hr]:border-slate-300
                  [&>pre]:bg-slate-100 [&>pre]:p-4 [&>pre]:rounded-xl [&>pre]:font-mono [&>pre]:text-sm [&>pre]:my-4 [&>pre]:overflow-x-auto
                  [&>table]:w-full [&>table]:border-collapse [&>table]:my-4
                  [&>table_th]:border [&>table_th]:border-slate-300 [&>table_th]:bg-slate-100 [&>table_th]:p-2.5 [&>table_th]:text-left [&>table_th]:font-semibold
                  [&>table_td]:border [&>table_td]:border-slate-300 [&>table_td]:p-2.5
                  [&>a]:text-teal-700 [&>a]:underline"
              />
            </div>
          ) : viewMode === "codigo" ? (
            /* Modo Código Fonte (HTML ou Markdown) */
            <div className="mx-auto max-w-[950px]">
              <div className="mb-2 flex items-center justify-between text-xs text-slate-500">
                <span>
                  Edição em código {formato === "html" ? "HTML" : "Markdown"}
                </span>
                <span>Quebras de linha preservadas</span>
              </div>
              <textarea
                ref={textareaRef}
                value={conteudo}
                onChange={(e) => {
                  setConteudo(e.target.value);
                  setModificado(true);
                }}
                className="field min-h-[580px] w-full resize-y rounded-2xl border-slate-300 bg-white p-5 font-mono text-sm leading-6 shadow-sm focus:border-teal-600 focus:ring-teal-200"
                placeholder={
                  formato === "html"
                    ? "<h1>Título</h1>\n<p>Digite seu HTML aqui...</p>"
                    : "# Título\n\nDigite seu Markdown aqui..."
                }
              />
            </div>
          ) : viewMode === "split" ? (
            /* Modo Lado a Lado (Split View) */
            <div className="grid h-full gap-4 lg:grid-cols-2">
              <div className="flex flex-col">
                <span className="mb-2 text-xs font-semibold text-slate-500">
                  Editor Markdown
                </span>
                <textarea
                  ref={textareaRef}
                  value={conteudo}
                  onChange={(e) => {
                    setConteudo(e.target.value);
                    setModificado(true);
                  }}
                  className="field flex-1 min-h-[550px] w-full resize-none rounded-2xl border-slate-300 bg-white p-4 font-mono text-sm leading-6 shadow-sm"
                  placeholder="# Título\n\nTexto..."
                />
              </div>
              <div className="flex flex-col">
                <span className="mb-2 text-xs font-semibold text-slate-500">
                  Pré-visualização da Folha
                </span>
                <div className="flex-1 min-h-[550px] overflow-y-auto rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
                  <MarkdownText>{conteudo}</MarkdownText>
                </div>
              </div>
            </div>
          ) : (
            /* Modo Prévia da Folha */
            <div className="mx-auto min-h-[620px] max-w-[850px] rounded-sm border border-slate-200/90 bg-white p-8 shadow-xl ring-1 ring-slate-900/5 sm:p-14">
              {formato === "html" ? (
                <div
                  className="prose max-w-none text-slate-800 leading-relaxed
                    [&>h1]:text-3xl [&>h1]:font-bold [&>h1]:mb-4
                    [&>h2]:text-2xl [&>h2]:font-bold [&>h2]:mb-3
                    [&>h3]:text-xl [&>h3]:font-semibold [&>h3]:mb-2
                    [&>p]:mb-3.5 [&>p]:leading-7
                    [&>ul]:list-disc [&>ul]:pl-6 [&>ul]:mb-3.5
                    [&>ol]:list-decimal [&>ol]:pl-6 [&>ol]:mb-3.5
                    [&>blockquote]:border-l-4 [&>blockquote]:border-teal-600 [&>blockquote]:pl-4 [&>blockquote]:italic [&>blockquote]:my-4
                    [&>hr]:my-6 [&>hr]:border-slate-300
                    [&>pre]:bg-slate-100 [&>pre]:p-4 [&>pre]:rounded-xl [&>pre]:font-mono [&>pre]:text-sm
                    [&>table]:w-full [&>table]:border-collapse [&>table]:my-4
                    [&>table_th]:border [&>table_th]:border-slate-300 [&>table_th]:bg-slate-100 [&>table_th]:p-2.5
                    [&>table_td]:border [&>table_td]:border-slate-300 [&>table_td]:p-2.5
                    [&>a]:text-teal-700 [&>a]:underline"
                  dangerouslySetInnerHTML={{ __html: conteudo }}
                />
              ) : (
                <MarkdownText>{conteudo}</MarkdownText>
              )}
            </div>
          )}
        </main>

        {/* Rodapé do Editor: Contadores e Dicas */}
        <footer className="flex flex-wrap items-center justify-between gap-3 border-t border-slate-200 bg-slate-50 px-4 py-2.5 text-xs text-slate-500 sm:px-6">
          <div className="flex items-center gap-3">
            <span>
              <strong>{estatisticas.palavras}</strong> palavras
            </span>
            <span>•</span>
            <span>
              <strong>{estatisticas.caracteres}</strong> caracteres
            </span>
            <span>•</span>
            <span className="uppercase font-semibold tracking-wider text-teal-800">
              {formato}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <span className="hidden sm:inline">
              Pressione <kbd className="rounded border bg-white px-1 py-0.5 font-mono text-[10px]">Ctrl+S</kbd> para salvar
            </span>
            <Button
              type="button"
              variant="secondary"
              disabled={salvando || excluindo}
              onClick={fechar}
            >
              Fechar
            </Button>
            <Button
              type="button"
              loading={salvando}
              disabled={excluindo}
              onClick={() => void salvar()}
            >
              <Save className="size-3.5" /> Salvar
            </Button>
          </div>
        </footer>
      </div>
    </div>
  );
}
