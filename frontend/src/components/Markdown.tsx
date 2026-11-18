import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'

/**
 * Renders user-authored Markdown (GFM: tables, task lists, strikethrough).
 *
 * `react-markdown` does not render raw HTML by default and sanitises link
 * URLs, so issue, PR, review, comment and release bodies cannot inject markup.
 */
export function Markdown({
  source,
  className = '',
}: {
  source: string | null | undefined
  className?: string
}) {
  if (!source) return null
  return (
    <div className={`markdown-body ${className}`}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ href, children }) => (
            <a href={href} target="_blank" rel="noopener noreferrer">
              {children}
            </a>
          ),
        }}
      >
        {source}
      </ReactMarkdown>
    </div>
  )
}
