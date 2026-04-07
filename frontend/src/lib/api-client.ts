import ky from 'ky'
import { getApiBaseUrl } from '@/lib/runtime-config'
import { useAuthStore } from '@/stores/auth-store'
import { useTeamStore } from '@/stores/team-store'

const retry = {
  limit: 1,
  methods: ['get'],
  statusCodes: [408, 502, 503, 504],
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
 * - Remote API base: `prefixUrl` + path without leading slash (`api/...`).
 * - Same-origin (browser / Vite): root-relative `/${path}` so the request stays on the dev
 *   origin and `beforeRequest` hooks attach `Authorization` reliably (absolute URLs + `ky`
 *   have regressed in Playwright/Chromium E2E to missing Bearer → 401 + hard redirect).
 */
export function apiPath(pathFromRoot: string): string {
  const normalized = pathFromRoot.replace(/^\//, '')
  const remote = kyPrefixUrl()
  if (remote) {
    return normalized
  }
  return `/${normalized}`
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
        const teamId = useTeamStore.getState().currentTeamId
        const url = request.url
        if (
          teamId != null &&
          /\/api\/v1\/(?!auth\/)/.test(url)
        ) {
          request.headers.set('X-Team-Id', String(teamId))
        }
      },
    ],
    afterResponse: [
      (_request, _options, response) => {
        if (response.status !== 401) {
          return
        }
        // Session may not be rehydrated yet; unauthenticated probes must not clear persist / hard-redirect.
        if (!useAuthStore.persist.hasHydrated()) {
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
})
