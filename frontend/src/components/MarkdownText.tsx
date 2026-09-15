import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";

export function MarkdownText({ children }: { children: string }) {
  return <div className="markdown-text"><Markdown remarkPlugins={[remarkGfm]} skipHtml>{children}</Markdown></div>;
}
