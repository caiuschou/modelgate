import { Card } from '@/components/ui/card'
import type {
  ParsedChatCompletionRequest,
  ParsedChatCompletionResponse,
  ParsedChatMessage,
  ParsedToolCall,
} from '@/features/logs/parse-chat-completion-display'

function tryFormatJson(s: string): string {
  const t = s.trim()
  if (!t) return s
  try {
    return JSON.stringify(JSON.parse(t), null, 2)
  } catch {
    return s
  }
}

const codeBlockClass =
  'min-w-0 max-w-full overflow-x-auto whitespace-pre-wrap break-words [overflow-wrap:anywhere] font-mono text-xs leading-relaxed'

function ToolCallCard({ tc, index }: { tc: ParsedToolCall; index: number }) {
  const title = tc.name || `工具调用 #${index + 1}`
  return (
    <Card className="min-w-0 max-w-full overflow-hidden border-border/80 p-3">
      <div className="flex min-w-0 flex-wrap items-baseline justify-between gap-2">
        <h3 className="min-w-0 break-words font-mono text-sm font-medium">{title}</h3>
        {tc.id ? (
          <span className="max-w-full break-all font-mono text-xs text-muted-foreground">
            {tc.id}
          </span>
        ) : null}
      </div>
      {tc.arguments.trim() ? (
        <pre className={`mt-2 max-h-56 max-w-full rounded-md bg-muted/50 p-2 ${codeBlockClass}`}>
          {tryFormatJson(tc.arguments)}
        </pre>
      ) : (
        <p className="mt-2 text-xs text-muted-foreground">（无参数）</p>
      )}
    </Card>
  )
}

function roleBadgeClass(role: string): string {
  switch (role) {
    case 'system':
      return 'bg-muted text-muted-foreground'
    case 'user':
      return 'bg-sky-500/15 text-sky-800 dark:text-sky-200'
    case 'assistant':
      return 'bg-emerald-500/15 text-emerald-800 dark:text-emerald-200'
    case 'tool':
      return 'bg-amber-500/15 text-amber-900 dark:text-amber-200'
    case 'developer':
      return 'bg-violet-500/15 text-violet-800 dark:text-violet-200'
    default:
      return 'bg-muted text-foreground'
  }
}

function RequestMessageCard({ msg, index }: { msg: ParsedChatMessage; index: number }) {
  const label = `#${index + 1}`
  const showEmptyHint =
    !msg.content.trim() &&
    !msg.reasoning.trim() &&
    !msg.refusal.trim() &&
    msg.tool_calls.length === 0
  return (
    <Card className="min-w-0 max-w-full overflow-hidden border-border/80 p-3">
      <div className="flex min-w-0 flex-wrap items-center gap-2">
        <span className="font-mono text-xs text-muted-foreground">{label}</span>
        <span
          className={`rounded px-2 py-0.5 font-mono text-xs font-medium ${roleBadgeClass(msg.role)}`}
        >
          {msg.role}
        </span>
        {msg.name ? (
          <span className="text-xs text-muted-foreground">
            name: <span className="font-mono">{msg.name}</span>
          </span>
        ) : null}
        {msg.role === 'tool' && msg.tool_call_id ? (
          <span className="max-w-full break-all font-mono text-xs text-muted-foreground">
            tool_call_id: {msg.tool_call_id}
          </span>
        ) : null}
      </div>

      {msg.reasoning.trim() ? (
        <details className="mt-2 min-w-0 max-w-full overflow-hidden rounded-md border border-border/60 bg-muted/20">
          <summary className="cursor-pointer select-none px-2 py-1.5 text-xs font-medium">
            推理 / 思考内容
            <span className="ml-1 font-normal text-muted-foreground">
              （{msg.reasoning.length.toLocaleString()} 字符）
            </span>
          </summary>
          <pre
            className={`max-h-64 border-t border-border/60 px-2 py-2 ${codeBlockClass}`}
          >
            {msg.reasoning}
          </pre>
        </details>
      ) : null}

      {msg.refusal.trim() ? (
        <div className="mt-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-sm">
          <p className="text-xs font-medium text-amber-900 dark:text-amber-200">拒绝</p>
          <p className="mt-1 whitespace-pre-wrap break-words [overflow-wrap:anywhere] text-xs">
            {msg.refusal}
          </p>
        </div>
      ) : null}

      {msg.content.trim() ? (
        <div className="mt-2 min-w-0">
          <p className="mb-1 text-xs font-medium text-muted-foreground">内容</p>
          <pre className={`max-h-96 rounded-md bg-muted/50 p-2 ${codeBlockClass}`}>
            {msg.content}
          </pre>
        </div>
      ) : showEmptyHint ? (
        <p className="mt-2 text-xs text-muted-foreground">（无文本内容）</p>
      ) : null}

      {msg.tool_calls.length > 0 ? (
        <div className="mt-2 min-w-0 space-y-2">
          <p className="text-xs font-medium text-muted-foreground">工具调用</p>
          <div className="space-y-2">
            {msg.tool_calls.map((tc, i) => (
              <ToolCallCard key={`${tc.id ?? 'tc'}-${i}`} tc={tc} index={i} />
            ))}
          </div>
        </div>
      ) : null}
    </Card>
  )
}

export function ChatCompletionResponseStructured({
  parsed,
}: {
  parsed: ParsedChatCompletionResponse
}) {
  const sourceLabel =
    parsed.source === 'sse_merged'
      ? '由 SSE 增量合并（流式）'
      : '非流式 JSON'

  return (
    <div className="min-w-0 max-w-full space-y-4">
      <p className="break-words text-xs text-muted-foreground">{sourceLabel}</p>

      {parsed.reasoning.trim() ? (
        <details className="min-w-0 max-w-full overflow-hidden rounded-lg border border-border/80 bg-muted/20">
          <summary className="cursor-pointer select-none px-3 py-2 text-sm font-medium">
            推理内容
            <span className="ml-2 font-normal text-muted-foreground">
              （{parsed.reasoning.length.toLocaleString()} 字符）
            </span>
          </summary>
          <pre
            className={`max-h-96 border-t border-border/60 px-3 py-2 ${codeBlockClass}`}
          >
            {parsed.reasoning}
          </pre>
        </details>
      ) : null}

      {parsed.refusal.trim() ? (
        <div className="min-w-0 max-w-full rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
          <p className="font-medium text-amber-900 dark:text-amber-200">拒绝回答</p>
          <p className="mt-1 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
            {parsed.refusal}
          </p>
        </div>
      ) : null}

      {parsed.content.trim() ? (
        <div className="min-w-0 max-w-full">
          <p className="mb-1.5 text-xs font-medium text-muted-foreground">助手正文</p>
          <div className="min-w-0 rounded-lg border border-border/80 bg-background p-3">
            <pre className={`max-h-96 ${codeBlockClass}`}>{parsed.content}</pre>
          </div>
        </div>
      ) : null}

      {parsed.tool_calls.length > 0 ? (
        <div className="min-w-0 max-w-full space-y-2">
          <p className="break-words text-xs font-medium text-muted-foreground">
            工具调用（模型发起，执行结果见下一轮请求的「工具结果」或同一会话后续日志）
          </p>
          <div className="space-y-3">
            {parsed.tool_calls.map((tc, i) => (
              <ToolCallCard key={`${tc.id ?? 'tc'}-${i}`} tc={tc} index={i} />
            ))}
          </div>
        </div>
      ) : null}
    </div>
  )
}

export function ChatCompletionRequestStructured({
  parsed,
}: {
  parsed: ParsedChatCompletionRequest
}) {
  return (
    <div className="min-w-0 max-w-full space-y-3">
      <p className="break-words text-xs text-muted-foreground">
        请求体中的 <span className="font-mono">messages</span> 数组（顺序与 API 一致；含 user / system /
        assistant / tool 等）。
      </p>
      <div className="space-y-3">
        {parsed.messages.map((msg, i) => (
          <RequestMessageCard key={`${msg.role}-${i}`} msg={msg} index={i} />
        ))}
      </div>
    </div>
  )
}
