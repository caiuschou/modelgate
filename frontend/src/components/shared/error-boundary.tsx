import { Component, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'

interface ErrorBoundaryProps {
  children: ReactNode
}

interface ErrorBoundaryState {
  hasError: boolean
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  constructor(props: ErrorBoundaryProps) {
    super(props)
    this.state = { hasError: false }
  }

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true }
  }

  componentDidCatch(error: Error) {
    console.error('Unhandled UI error:', error)
  }

  handleReload = () => {
    window.location.reload()
  }

  render() {
    if (this.state.hasError) {
      return (
        <main className="flex min-h-screen items-center justify-center bg-background p-4">
          <section className="w-full max-w-md rounded-lg border border-border bg-card p-6">
            <h1 className="text-lg font-semibold">页面出现异常</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              请刷新页面重试。如果持续出现，请联系管理员。
            </p>
            <Button className="mt-4 w-full" onClick={this.handleReload}>
              刷新页面
            </Button>
          </section>
        </main>
      )
    }

    return this.props.children
  }
}
