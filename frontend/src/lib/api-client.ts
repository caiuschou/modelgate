import ky from 'ky'
import { getApiBaseUrl } from '@/lib/runtime-config'
import { useAuthStore } from '@/stores/auth-store'

const retry = {
  limit: 1,
  methods: ['get'] as const,
  statusCodes: [408, 502, 503, 504] as const,
}

function kyPrefixUrl(): string | undefined {
  const base = getApiBaseUrl().trim()
  if (!base) {
    return undefined
  }
  return base.replace(/\/$/, '')
}

/**
 * ky forbids a leading slash on the request path when `prefixUrl` is set (including `''`).
 * - Same-origin / dev proxy: no `prefixUrl` → use root-absolute path (`/api/...`).
 * - Remote API base: `prefixUrl` + path without leading slash (`api/...`).
 */
export function apiPath(pathFromRoot: string): string {
  const normalized = pathFromRoot.replace(/^\//, '')
  return kyPrefixUrl() ? normalized : `/${normalized}`
}

const prefixUrlForKy = kyPrefixUrl()
const kyShared = {
  ...(prefixUrlForKy ? { prefixUrl: prefixUrlForKy } : {}),
  timeout: 30_000,
  retry,
}

/** Login / register — no Bearer injection, no 401 → full-page redirect (avoids loops on failed login). */
export const publicApi = ky.create({
  ...kyShared,
})

export const apiClient = ky.create({
  ...kyShared,
  hooks: {
    beforeRequest: [
      (request) => {
        const token = useAuthStore.getState().token
        if (token) {
          request.headers.set('Authorization', `Bearer ${token}`)
        }
      },
    ],
    afterResponse: [
      (_request, _options, response) => {
        if (response.status !== 401) {
          return
        }
        const path = window.location.pathname
        if (path === '/login' || path === '/register') {
          return
        }
        useAuthStore.getState().logout()
        const redirectTo = encodeURIComponent(
          `${window.location.pathname}${window.location.search}`,
        )
        window.location.href = `/login?redirect=${redirectTo}`
      },
    ],
  },
  retry: {
    limit: 1,
    methods: ['get'],
    statusCodes: [408, 502, 503, 504],
  },
})
