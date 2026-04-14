import { QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from 'react-router-dom'
import { ErrorBoundary } from '@/components/shared/error-boundary'
import { useAuditLogWebSocket } from '@/hooks/use-audit-log-ws'
import { useTheme } from '@/hooks/use-theme'
import { queryClient } from '@/lib/query-client'
import { router } from '@/routes'

/** Must render under `QueryClientProvider` (uses `useQueryClient`). */
function AuditLogWebSocketSubscriber() {
  useAuditLogWebSocket()
  return null
}

export function App() {
  useTheme()

  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <AuditLogWebSocketSubscriber />
        <RouterProvider router={router} />
      </QueryClientProvider>
    </ErrorBoundary>
  )
}
