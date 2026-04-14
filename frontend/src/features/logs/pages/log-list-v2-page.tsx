import { LogListPage } from '@/features/logs/pages/log-list-page'

/** 新版日志列表：与 {@link LogListPage} 共用筛选与接口，表格优先展示会话 (thread)。 */
export function LogListV2Page() {
  return <LogListPage listVariant="v2" />
}
