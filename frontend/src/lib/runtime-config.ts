const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? ''

export function getApiBaseUrl(): string {
  return API_BASE_URL
}

/** Called from main so Vite always inlines VITE_* (unused env modules are dropped). */
export function applyRuntimeConfigToDocument(): void {
  document.documentElement.dataset.apiBaseUrl = API_BASE_URL
}
