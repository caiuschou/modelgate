interface LoadingSkeletonProps {
  lines?: number
}

export function LoadingSkeleton({ lines = 4 }: LoadingSkeletonProps) {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="h-5 w-40 animate-pulse rounded bg-accent" />
      <div className="mt-4 space-y-2">
        {Array.from({ length: lines }).map((_, index) => (
          <div
            key={`line-${index}`}
            className="h-4 w-full animate-pulse rounded bg-accent"
          />
        ))}
      </div>
    </section>
  )
}
