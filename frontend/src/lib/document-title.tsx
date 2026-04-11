import { useEffect } from 'react'
import { useMatches } from 'react-router-dom'

export const APP_DISPLAY_NAME = 'ModelGate'

export function formatPageTitle(suffix?: string) {
  return suffix ? `${APP_DISPLAY_NAME} - ${suffix}` : APP_DISPLAY_NAME
}

/** Reads `handle.pageTitle` from the deepest matching route and sets `document.title`. */
export function DocumentTitleSync() {
  const matches = useMatches()

  useEffect(() => {
    const match = [...matches]
      .reverse()
      .find((m) => typeof (m.handle as { pageTitle?: string })?.pageTitle === 'string')
    const suffix = (match?.handle as { pageTitle?: string } | undefined)?.pageTitle
    document.title = formatPageTitle(suffix)
  }, [matches])

  return null
}
