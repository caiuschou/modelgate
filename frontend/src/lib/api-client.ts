import ky from 'ky'
import { getApiBaseUrl } from '@/lib/runtime-config'
import { useAuthStore } from '@/stores/auth-store'

export const apiClient = ky.create({
  prefixUrl: getApiBaseUrl(),
  timeout: 30_000,
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
        if (response.status === 401) {
          useAuthStore.getState().logout()
          const redirectTo = encodeURIComponent(
            `${window.location.pathname}${window.location.search}`,
          )
          window.location.href = `/login?redirect=${redirectTo}`
        }
      },
    ],
  },
  retry: {
    limit: 1,
    methods: ['get'],
    statusCodes: [408, 502, 503, 504],
  },
})
