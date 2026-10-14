import type { ReactNode } from "react";
import type { DescriptionBlock } from "../../bindings/DescriptionBlock";
import type { DescriptionInline } from "../../bindings/DescriptionInline";
import type { JobDescription } from "../../bindings/JobDescription";

function httpUrl(value: string | null): string | undefined {
  if (!value) return undefined;
  try {
    const parsed = new URL(value);
    if (parsed.protocol === "http:" || parsed.protocol === "https:") return parsed.toString();
  } catch {
    return undefined;
  }
  return undefined;
}

function inlineNodes(
  nodes: DescriptionInline[],
  onOpenUrl: (url: string) => void,
  prefix: string,
): ReactNode[] {
  return nodes.map((node, index) => {
    const key = `${prefix}-${index}`;
    switch (node.kind) {
      case "text":
        return <span key={key}>{node.text}</span>;
      case "strong":
        return <strong key={key}>{inlineNodes(node.children, onOpenUrl, key)}</strong>;
      case "emphasis":
        return <em key={key}>{inlineNodes(node.children, onOpenUrl, key)}</em>;
      case "link": {
        const url = httpUrl(node.url);
        return url ? (
          <a
            key={key}
            href={url}
            className="underline underline-offset-2"
            onClick={(event) => {
              event.preventDefault();
              onOpenUrl(url);
            }}
          >
            {node.text}
          </a>
        ) : (
          <span key={key}>{node.text}</span>
        );
      }
    }
  });
}

function renderBlock(
  block: DescriptionBlock,
  index: number,
  onOpenUrl: (url: string) => void,
): ReactNode {
  const key = `description-${index}`;
  switch (block.kind) {
    case "heading": {
      const Heading = (["h1", "h2", "h3", "h4", "h5", "h6"] as const)[
        Math.max(0, Math.min(5, block.level - 1))
      ];
      return (
        <Heading key={key} className="font-bold leading-snug">
          {inlineNodes(block.children, onOpenUrl, key)}
        </Heading>
      );
    }
    case "paragraph":
      return (
        <p key={key} className="whitespace-pre-wrap break-words leading-relaxed">
          {inlineNodes(block.children, onOpenUrl, key)}
        </p>
      );
    case "list": {
      const List = block.ordered ? "ol" : "ul";
      return (
        <List key={key} className={block.ordered ? "list-decimal pl-6" : "list-disc pl-6"}>
          {block.items.map((item, itemIndex) => (
            <li key={`${key}-${itemIndex}`} className="break-words leading-relaxed">
              {inlineNodes(item, onOpenUrl, `${key}-${itemIndex}`)}
            </li>
          ))}
        </List>
      );
    }
    case "break":
      return <br key={key} />;
  }
}

export function JobDescriptionView({
  description,
  fallback,
  onOpenUrl,
}: {
  description?: JobDescription | null;
  fallback?: string | null;
  onOpenUrl: (url: string) => void;
}) {
  if (!description?.blocks.length) {
    return (
      <p className="whitespace-pre-wrap break-words leading-relaxed">
        {description?.plain_text || fallback || "No description extracted."}
      </p>
    );
  }
  return (
    <div className="grid min-w-0 gap-3 break-words text-sm leading-relaxed">
      {description.blocks.map((block, index) => renderBlock(block, index, onOpenUrl))}
      {description.truncated && <p className="text-xs text-[var(--color-ink-mute)]">Description shortened for display.</p>}
    </div>
  );
}
