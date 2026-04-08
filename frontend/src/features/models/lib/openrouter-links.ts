export function openRouterModelHref(id: string): string {
  return `https://openrouter.ai/${id
    .split('/')
    .map(encodeURIComponent)
    .join('/')}`
}
