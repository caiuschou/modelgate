import { useEffect } from 'react'
import { useUiStore } from '@/stores/ui-store'

function getSystemTheme(): 'light' | 'dark' {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
    ? 'dark'
    : 'light'
}

export function useTheme() {
  const theme = useUiStore((state) => state.theme)

  useEffect(() => {
    const root = document.documentElement
    const resolvedTheme = theme === 'system' ? getSystemTheme() : theme
    root.classList.toggle('dark', resolvedTheme === 'dark')
  }, [theme])
}
