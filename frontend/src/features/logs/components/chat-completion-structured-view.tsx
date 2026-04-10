import { Card } from '@/components/ui/card'
import type {
  ParsedChatCompletionRequest,
  ParsedChatCompletionResponse,
  ParsedToolCall,
  ParsedToolResultMessage,
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

function ToolCallCard({ tc, index }: { tc: ParsedToolCall; index: number }) {
  const title = tc.name || `工具调用 #${index + 1}`
  return (
    <Card className="overflow-hidden border-border/80 p-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="font-mono text-sm font-medium">{title}</h3>
        {tc.id ? (
          <span className="font-mono text-xs text-muted-foreground">{tc.id}</span>
        ) : null}
      </div>
      {tc.arguments.trim() ? (
        <pre className="mt-2 max-h-56 overflow-auto rounded-md bg-muted/50 p-2 font-mono text-xs leading-relaxed">
          {tryFormatJson(tc.arguments)}
        </pre>
      ) : (
        <p className="mt-2 text-xs text-muted-foreground">（无参数）</p>
      )}
    </Card>
  )
}

function ToolResultCard({ tm, index }: { tm: ParsedToolResultMessage; index: number }) {
  return (
    <Card className="overflow-hidden border-border/80 p-3">
      <div className="flex flex-wrap items-baseline justify-between gap-2">
        <h3 className="text-sm font-medium">工具结果 #{index + 1}</h3>
        {tm.tool_call_id ? (
          <span className="font-mono text-xs text-muted-foreground">
            {tm.tool_call_id}
          </span>
        ) : null}
      </div>
      <pre className="mt-2 max-h-56 overflow-auto whitespace-pre-wrap break-words rounded-md bg-muted/50 p-2 font-mono text-xs leading-relaxed">
        {tryFormatJson(tm.content)}
      </pre>
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
    <div className="space-y-4">
      <p className="text-xs text-muted-foreground">{sourceLabel}</p>

      {parsed.reasoning.trim() ? (
        <details className="rounded-lg border border-border/80 bg-muted/20">
          <summary className="cursor-pointer select-none px-3 py-2 text-sm font-medium">
            推理内容
            <span className="ml-2 font-normal text-muted-foreground">
              （{parsed.reasoning.length.toLocaleString()} 字符）
            </span>
          </summary>
          <pre className="max-h-96 overflow-auto whitespace-pre-wrap break-words border-t border-border/60 px-3 py-2 font-mono text-xs leading-relaxed">
            {parsed.reasoning}
          </pre>
        </details>
      ) : null}

      {parsed.refusal.trim() ? (
        <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
          <p className="font-medium text-amber-900 dark:text-amber-200">拒绝回答</p>
          <p className="mt-1 whitespace-pre-wrap">{parsed.refusal}</p>
        </div>
      ) : null}

      {parsed.content.trim() ? (
        <div>
          <p className="mb-1.5 text-xs font-medium text-muted-foreground">助手正文</p>
          <div className="rounded-lg border border-border/80 bg-background p-3">
            <pre className="max-h-96 overflow-auto whitespace-pre-wrap break-words font-mono text-xs leading-relaxed">
              {parsed.content}
            </pre>
          </div>
        </div>
      ) : null}

      {parsed.tool_calls.length > 0 ? (
        <div className="space-y-2">
          <p className="text-xs font-medium text-muted-foreground">
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
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        请求中携带的 <span className="font-mono">role: tool</span> 消息（上一轮工具执行结果）
      </p>
      <div className="space-y-3">
        {parsed.tool_messages.map((tm, i) => (
          <ToolResultCard key={`${tm.tool_call_id ?? 'tm'}-${i}`} tm={tm} index={i} />
        ))}
      </div>
    </div>
  )
}
