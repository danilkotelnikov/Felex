import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { cn } from '@/lib/utils';
import { openExternalUrl } from '@/lib/desktop';

type MarkdownVariant = 'chat' | 'document';

interface MarkdownContentProps {
  markdown: string;
  variant?: MarkdownVariant;
  className?: string;
}

export function MarkdownContent({
  markdown,
  variant = 'document',
  className,
}: MarkdownContentProps) {
  const styles = variant === 'chat'
    ? {
        root: 'space-y-3 text-[13px] text-[--text-primary]',
        h1: 'mb-3 text-base font-semibold text-[--text-primary]',
        h2: 'mt-5 mb-2 text-sm font-semibold text-[--text-primary]',
        h3: 'mt-4 mb-2 text-[11px] font-semibold uppercase tracking-wide text-[--text-secondary]',
        p: 'leading-6 text-[--text-primary]',
        ul: 'list-disc pl-5 space-y-1 text-[--text-primary]',
        ol: 'list-decimal pl-5 space-y-1 text-[--text-primary]',
        li: 'leading-6',
        table: 'min-w-full border-collapse text-[11px]',
        thead: 'bg-[--bg-hover]',
        th: 'border border-[--border] px-2.5 py-2 text-left font-medium',
        td: 'border border-[--border] px-2.5 py-2 align-top',
        code: 'rounded bg-[--bg-hover] px-1.5 py-0.5 text-[11px] text-[--text-primary]',
        pre: 'overflow-auto rounded-[--radius-md] bg-[--bg-base] p-3 text-[11px] text-[--text-primary]',
        quote: 'border-l-2 border-[--accent] pl-3 text-[--text-secondary]',
      }
    : {
        root: 'space-y-4 text-sm text-[--text-primary]',
        h1: 'mb-4 text-2xl font-semibold text-[--text-primary]',
        h2: 'mt-8 mb-3 text-lg font-semibold text-[--text-primary]',
        h3: 'mt-6 mb-2 text-sm font-semibold uppercase tracking-wide text-[--text-secondary]',
        p: 'leading-6 text-[--text-primary]',
        ul: 'list-disc pl-5 space-y-1 text-[--text-primary]',
        ol: 'list-decimal pl-5 space-y-1 text-[--text-primary]',
        li: 'leading-6',
        table: 'min-w-full border-collapse text-xs',
        thead: 'bg-[--bg-hover]',
        th: 'border border-[--border] px-3 py-2 text-left font-medium',
        td: 'border border-[--border] px-3 py-2 align-top',
        code: 'rounded bg-[--bg-hover] px-1.5 py-0.5 text-[12px] text-[--text-primary]',
        pre: 'overflow-auto rounded-[--radius-md] bg-[--bg-base] p-3 text-xs text-[--text-primary]',
        quote: 'border-l-2 border-[--accent] pl-3 text-[--text-secondary]',
      };

  return (
    <div className={cn(styles.root, className)}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <h1 className={styles.h1}>{children}</h1>,
          h2: ({ children }) => <h2 className={styles.h2}>{children}</h2>,
          h3: ({ children }) => <h3 className={styles.h3}>{children}</h3>,
          p: ({ children }) => <p className={styles.p}>{children}</p>,
          ul: ({ children }) => <ul className={styles.ul}>{children}</ul>,
          ol: ({ children }) => <ol className={styles.ol}>{children}</ol>,
          li: ({ children }) => <li className={styles.li}>{children}</li>,
          table: ({ children }) => (
            <div className="overflow-x-auto">
              <table className={styles.table}>{children}</table>
            </div>
          ),
          thead: ({ children }) => <thead className={styles.thead}>{children}</thead>,
          th: ({ children }) => <th className={styles.th}>{children}</th>,
          td: ({ children }) => <td className={styles.td}>{children}</td>,
          code: ({ children }) => <code className={styles.code}>{children}</code>,
          pre: ({ children }) => <pre className={styles.pre}>{children}</pre>,
          blockquote: ({ children }) => <blockquote className={styles.quote}>{children}</blockquote>,
          a: ({ href, children }) => (
            <a
              href={href}
              className="text-[--accent] underline underline-offset-2"
              onClick={(event) => {
                if (!href || href.startsWith('#') || href.startsWith('/')) {
                  return;
                }

                event.preventDefault();
                void openExternalUrl(href).catch((error) => {
                  console.error('Failed to open markdown link:', error);
                });
              }}
            >
              {children}
            </a>
          ),
        }}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  );
}
