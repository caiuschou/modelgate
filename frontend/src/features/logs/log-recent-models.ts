const STORAGE_KEY = 'modelgate.logRecentModels'
const MAX_ITEMS = 48

export type RecentModelEntry = {
  name: string
  /** Unix ms */
  lastUsedAt: number
}

function persist(entries: RecentModelEntry[]): void {
  if (typeof window === 'undefined') return
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(entries))
  } catch {
    // quota or privacy mode
  }
}

function parseLegacyStringArray(parsed: string[]): RecentModelEntry[] {
  const now = Date.now()
  const out: RecentModelEntry[] = []
  const seen = new Set<string>()
  for (let i = 0; i < parsed.length; i++) {
    const name = String(parsed[i]).trim()
    if (!name || seen.has(name)) continue
    seen.add(name)
    out.push({ name, lastUsedAt: now - i })
  }
  return out
}

function parseEntries(parsed: unknown): RecentModelEntry[] {
  if (!Array.isArray(parsed) || parsed.length === 0) return []
  const first = parsed[0]
  if (typeof first === 'string') {
    return parseLegacyStringArray(parsed as string[])
  }
  const out: RecentModelEntry[] = []
  const seen = new Set<string>()
  for (const x of parsed) {
    if (x === null || typeof x !== 'object') continue
    const rec = x as Record<string, unknown>
    const name = typeof rec.name === 'string' ? rec.name.trim() : ''
    const lastUsedAt =
      typeof rec.lastUsedAt === 'number' && Number.isFinite(rec.lastUsedAt)
        ? rec.lastUsedAt
        : 0
    if (!name || seen.has(name)) continue
    seen.add(name)
    out.push({ name, lastUsedAt })
  }
  return out.sort((a, b) => b.lastUsedAt - a.lastUsedAt)
}

/** Sorted by `lastUsedAt` descending (most recent first). */
export function readRecentLogModels(): RecentModelEntry[] {
  if (typeof window === 'undefined') return []
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (raw === null || raw === '') return []
    const parsed = JSON.parse(raw) as unknown
    return parseEntries(parsed)
  } catch {
    return []
  }
}

export function filterRecentModelsByQuery(
  entries: RecentModelEntry[],
  query: string,
): RecentModelEntry[] {
  const q = query.trim().toLowerCase()
  if (!q) return entries
  return entries.filter((e) => e.name.toLowerCase().includes(q))
}

/** Updates `lastUsedAt` to now; keeps list sorted by recency when persisted. */
export function rememberLogModel(model: string): void {
  if (typeof window === 'undefined') return
  const trimmed = model.trim()
  if (!trimmed) return
  const now = Date.now()
  let entries = readRecentLogModels().filter((e) => e.name !== trimmed)
  entries.unshift({ name: trimmed, lastUsedAt: now })
  entries = entries.slice(0, MAX_ITEMS)
  persist(entries)
}
