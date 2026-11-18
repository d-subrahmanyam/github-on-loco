import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Markdown } from './Markdown'

describe('Markdown', () => {
  it('renders GFM headings, task lists and tables', () => {
    render(
      <Markdown
        source={'# Release notes\n\n- [x] shipped\n\n| a | b |\n|---|---|\n| 1 | 2 |'}
      />,
    )
    expect(screen.getByRole('heading', { name: 'Release notes' })).toBeInTheDocument()
    expect(screen.getByRole('checkbox')).toBeChecked()
    expect(screen.getByRole('table')).toBeInTheDocument()
  })

  it('does not render raw HTML or javascript: links', () => {
    const { container } = render(
      <Markdown
        source={'<img src=x onerror="alert(1)">\n\n[click](javascript:alert(1))'}
      />,
    )
    expect(container.querySelector('img')).toBeNull()
    const link = container.querySelector('a')
    expect(link).not.toBeNull()
    expect(link?.getAttribute('href')).not.toContain('javascript:')
  })

  it('renders nothing for an empty body', () => {
    const { container } = render(<Markdown source={null} />)
    expect(container).toBeEmptyDOMElement()
  })
})
