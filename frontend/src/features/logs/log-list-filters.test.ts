import { describe, expect, it } from 'vitest'
import {
  auditLogListQuery,
  buildAppliedSearchParams,
  parseUnixSearchParam,
  urlHasAdvancedFilters,
} from './log-list-filters'

describe('parseUnixSearchParam', () => {
  it('returns fallback for null, empty, or non-numeric', () => {
    const fb = 100
    expect(parseUnixSearchParam(null, fb)).toBe(100)
    expect(parseUnixSearchParam('', fb)).toBe(100)
    expect(parseUnixSearchParam('not-a-number', fb)).toBe(100)
  })

  it('parses integer and float strings', () => {
    expect(parseUnixSearchParam('1711843200', 0)).toBe(1711843200)
    expect(parseUnixSearchParam('3.14', 0)).toBe(3.14)
  })
})

describe('urlHasAdvancedFilters', () => {
  it('is false when only keyword or model is set', () => {
    expect(urlHasAdvancedFilters(new URLSearchParams('keyword=x'))).toBe(false)
    expect(urlHasAdvancedFilters(new URLSearchParams('model=gpt'))).toBe(false)
    expect(urlHasAdvancedFilters(new URLSearchParams('keyword=x&model=y'))).toBe(false)
  })

  it('is true when any advanced key is non-empty', () => {
    expect(urlHasAdvancedFilters(new URLSearchParams('app_id=myapp'))).toBe(true)
    expect(urlHasAdvancedFilters(new URLSearchParams('finish_reason=stop'))).toBe(true)
    expect(urlHasAdvancedFilters(new URLSearchParams('status_code=200'))).toBe(true)
    expect(urlHasAdvancedFilters(new URLSearchParams('token_id=42'))).toBe(true)
  })

  it('treats empty advanced values as absent', () => {
    expect(urlHasAdvancedFilters(new URLSearchParams('app_id='))).toBe(false)
    expect(urlHasAdvancedFilters(new URLSearchParams('status_code='))).toBe(false)
  })
})

describe('auditLogListQuery', () => {
  const base = {
    startTime: 100,
    endTime: 200,
    limit: 20,
    offset: 0,
    keyword: '',
    model: '',
    appId: '',
    finishReason: '',
    statusCode: '',
    tokenId: '',
  }

  it('includes only time range and pagination when optional filters empty', () => {
    expect(auditLogListQuery(base)).toEqual({
      start_time: 100,
      end_time: 200,
      limit: 20,
      offset: 0,
    })
  })

  it('trims string filters and adds status_code and token_id when valid', () => {
    expect(
      auditLogListQuery({
        ...base,
        keyword: '  rid  ',
        model: ' m ',
        appId: ' app ',
        finishReason: ' stop ',
        statusCode: '429',
        tokenId: ' 7 ',
      }),
    ).toEqual({
      start_time: 100,
      end_time: 200,
      limit: 20,
      offset: 0,
      keyword: 'rid',
      model: 'm',
      app_id: 'app',
      finish_reason: 'stop',
      status_code: 429,
      token_id: 7,
    })
  })

  it('omits status_code when not a finite number', () => {
    const q = auditLogListQuery({ ...base, statusCode: 'bad' })
    expect(q).not.toHaveProperty('status_code')
  })

  it('omits token_id when not a finite number', () => {
    expect(auditLogListQuery({ ...base, tokenId: 'x' })).not.toHaveProperty('token_id')
  })
})

describe('buildAppliedSearchParams', () => {
  it('sets core keys and only non-empty optionals', () => {
    const sp = buildAppliedSearchParams({
      start: 1,
      end: 2,
      off: '40',
      kw: ' k ',
      m: '',
      app: 'aid',
      fr: '',
      sc: '500',
      tid: '',
    })
    expect(sp.get('start_time')).toBe('1')
    expect(sp.get('end_time')).toBe('2')
    expect(sp.get('offset')).toBe('40')
    expect(sp.get('keyword')).toBe('k')
    expect(sp.get('app_id')).toBe('aid')
    expect(sp.get('status_code')).toBe('500')
    expect(sp.get('model')).toBeNull()
    expect(sp.get('finish_reason')).toBeNull()
    expect(sp.get('token_id')).toBeNull()
  })
})
