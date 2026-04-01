import { QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from 'react-router-dom'
import { ErrorBoundary } from '@/components/shared/error-boundary'
import { useTheme } from '@/hooks/use-theme'
import { queryClient } from '@/lib/query-client'
import { router } from '@/routes'

export function App() {
  useTheme()

  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
