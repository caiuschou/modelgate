import { describe, expect, it } from 'vitest'
import {
  getHomeNavItem,
  isSidebarLinkActive,
  sidebarHref,
} from './sidebar-nav-config'

describe('sidebarHref', () => {
  it('returns direct path when token is present', () => {
    expect(sidebarHref('/api-keys', false, 'tok')).toBe('/api-keys')
  })

  it('returns direct path when guestOk is true without token', () => {
    expect(sidebarHref('/', true, null)).toBe('/')
  })

  it('returns login redirect when unauthenticated and not guestOk', () => {
    expect(sidebarHref('/api-keys', false, null)).toBe(
      '/login?redirect=%2Fapi-keys',
    )
  })
})

describe('getHomeNavItem', () => {
  it('points logged-in users to dashboard', () => {
    const item = getHomeNavItem('any-token')
    expect(item.to).toBe('/dashboard')
    expect(item.guestOk).toBe(false)
  })

  it('points guests to marketing home', () => {
    const item = getHomeNavItem(null)
    expect(item.to).toBe('/')
    expect(item.guestOk).toBe(true)
  })
})

describe('isSidebarLinkActive', () => {
  it('matches root only for home path', () => {
    expect(isSidebarLinkActive('/', '/')).toBe(true)
    expect(isSidebarLinkActive('/', '/dashboard')).toBe(false)
  })

  it('matches exact and nested paths for normal routes', () => {
    expect(isSidebarLinkActive('/api-keys', '/api-keys')).toBe(true)
    expect(isSidebarLinkActive('/teams', '/teams')).toBe(true)
    expect(isSidebarLinkActive('/teams', '/teams/42')).toBe(true)
    expect(isSidebarLinkActive('/teams', '/api-keys')).toBe(false)
  })

  it('treats /logs/v2 as distinct from /logs', () => {
    expect(isSidebarLinkActive('/logs/v2', '/logs/v2')).toBe(true)
    expect(isSidebarLinkActive('/logs/v2', '/logs')).toBe(false)
    expect(isSidebarLinkActive('/logs', '/logs/v2')).toBe(false)
    expect(isSidebarLinkActive('/logs', '/logs')).toBe(true)
    expect(isSidebarLinkActive('/logs', '/logs/audit/1')).toBe(true)
  })

  it('matches dashboard path', () => {
    expect(isSidebarLinkActive('/dashboard', '/dashboard')).toBe(true)
    expect(isSidebarLinkActive('/dashboard', '/')).toBe(false)
  })
})
