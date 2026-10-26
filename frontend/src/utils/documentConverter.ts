/* Developed with care by FACRF - https://github.com/facrf */

/**
 * Converte HTML básico para Markdown padrão
 */
export function htmlToMarkdown(html: string): string {
  if (!html.trim()) return "";

  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");

  function processNode(node: Node): string {
    if (node.nodeType === Node.TEXT_NODE) {
      return node.textContent || "";
    }
    if (node.nodeType !== Node.ELEMENT_NODE) {
      return "";
    }

    const el = node as HTMLElement;
    const tag = el.tagName.toLowerCase();
    const children = Array.from(el.childNodes).map(processNode).join("");

    switch (tag) {
      case "h1":
        return `\n# ${children.trim()}\n\n`;
      case "h2":
        return `\n## ${children.trim()}\n\n`;
      case "h3":
        return `\n### ${children.trim()}\n\n`;
      case "h4":
        return `\n#### ${children.trim()}\n\n`;
      case "p":
        return `\n${children.trim()}\n\n`;
      case "strong":
      case "b":
        return `**${children}**`;
      case "em":
      case "i":
        return `*${children}*`;
      case "u":
        return `<u>${children}</u>`;
      case "s":
      case "strike":
      case "del":
        return `~~${children}~~`;
      case "code":
        return `\`${children}\``;
      case "pre":
        return `\n\`\`\`\n${el.textContent || ""}\n\`\`\`\n\n`;
      case "blockquote":
        return `\n> ${children.trim().replace(/\n/g, "\n> ")}\n\n`;
      case "hr":
        return "\n---\n\n";
      case "br":
        return "\n";
      case "a": {
        const href = el.getAttribute("href") || "#";
        return `[${children || href}](${href})`;
      }
      case "ul": {
        const items = Array.from(el.children)
          .filter((c) => c.tagName.toLowerCase() === "li")
          .map((c) => `- ${Array.from(c.childNodes).map(processNode).join("").trim()}`)
          .join("\n");
        return `\n${items}\n\n`;
      }
      case "ol": {
        const items = Array.from(el.children)
          .filter((c) => c.tagName.toLowerCase() === "li")
          .map((c, i) => `${i + 1}. ${Array.from(c.childNodes).map(processNode).join("").trim()}`)
          .join("\n");
        return `\n${items}\n\n`;
      }
      case "table": {
        const rows = Array.from(el.querySelectorAll("tr"));
        if (!rows.length) return "";
        let md = "\n";
        rows.forEach((row, idx) => {
          const cells = Array.from(row.querySelectorAll("th, td")).map((cell) =>
            (cell.textContent || "").trim().replace(/\|/g, "\\|"),
          );
          md += `| ${cells.join(" | ")} |\n`;
          if (idx === 0) {
            md += `| ${cells.map(() => "---").join(" | ")} |\n`;
          }
        });
        return md + "\n";
      }
      default:
        return children;
    }
  }

  const result = Array.from(doc.body.childNodes).map(processNode).join("");
  return result.replace(/\n{3,}/g, "\n\n").trim();
}

/**
 * Converte Markdown simples para HTML estruturado para o editor visual
 */
export function markdownToHtml(md: string): string {
  if (!md.trim()) return "<p><br></p>";

  const lines = md.split(/\r?\n/);
  const htmlLines: string[] = [];
  let inCodeBlock = false;
  let codeBuffer: string[] = [];
  let inList: "ul" | "ol" | null = null;
  let inBlockquote = false;

  const closeList = () => {
    if (inList) {
      htmlLines.push(inList === "ul" ? "</ul>" : "</ol>");
      inList = null;
    }
  };

  const closeBlockquote = () => {
    if (inBlockquote) {
      htmlLines.push("</blockquote>");
      inBlockquote = false;
    }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Code block ```
    if (line.trim().startsWith("```")) {
      if (inCodeBlock) {
        htmlLines.push(`<pre><code>${codeBuffer.join("\n")}</code></pre>`);
        codeBuffer = [];
        inCodeBlock = false;
      } else {
        closeList();
        closeBlockquote();
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeBuffer.push(escapeHtml(line));
      continue;
    }

    // HR
    if (/^(\*\*\*|---|___)$/.test(line.trim())) {
      closeList();
      closeBlockquote();
      htmlLines.push("<hr>");
      continue;
    }

    // Headings
    const headingMatch = line.match(/^(#{1,4})\s+(.+)$/);
    if (headingMatch) {
      closeList();
      closeBlockquote();
      const level = headingMatch[1].length;
      htmlLines.push(`<h${level}>${formatInline(headingMatch[2])}</h${level}>`);
      continue;
    }

    // Blockquote
    if (line.startsWith("> ")) {
      closeList();
      if (!inBlockquote) {
        htmlLines.push("<blockquote>");
        inBlockquote = true;
      }
      htmlLines.push(`<p>${formatInline(line.slice(2))}</p>`);
      continue;
    } else {
      closeBlockquote();
    }

    // Unordered List
    const ulMatch = line.match(/^[-*]\s+(.+)$/);
    if (ulMatch) {
      if (inList !== "ul") {
        closeList();
        htmlLines.push("<ul>");
        inList = "ul";
      }
      htmlLines.push(`<li>${formatInline(ulMatch[1])}</li>`);
      continue;
    }

    // Ordered List
    const olMatch = line.match(/^\d+\.\s+(.+)$/);
    if (olMatch) {
      if (inList !== "ol") {
        closeList();
        htmlLines.push("<ol>");
        inList = "ol";
      }
      htmlLines.push(`<li>${formatInline(olMatch[1])}</li>`);
      continue;
    }

    closeList();

    // Empty line
    if (!line.trim()) {
      continue;
    }

    // Table line
    if (line.trim().startsWith("|") && line.trim().endsWith("|")) {
      htmlLines.push(`<p>${formatInline(line)}</p>`);
      continue;
    }

    // Regular paragraph
    htmlLines.push(`<p>${formatInline(line)}</p>`);
  }

  closeList();
  closeBlockquote();
  if (inCodeBlock) {
    htmlLines.push(`<pre><code>${codeBuffer.join("\n")}</code></pre>`);
  }

  return htmlLines.length > 0 ? htmlLines.join("\n") : "<p><br></p>";
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function formatInline(text: string): string {
  let res = escapeHtml(text);
  // Bold
  res = res.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
  res = res.replace(/__(.+?)__/g, "<strong>$1</strong>");
  // Italic
  res = res.replace(/\*(.+?)\*/g, "<em>$1</em>");
  res = res.replace(/_(.+?)_/g, "<em>$1</em>");
  // Strikethrough
  res = res.replace(/~~(.+?)~~/g, "<del>$1</del>");
  // Inline Code
  res = res.replace(/`([^`]+)`/g, "<code>$1</code>");
  // Links
  res = res.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>');
  return res;
}
