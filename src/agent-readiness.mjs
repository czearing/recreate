export function buildAgentReadiness({
  implementationBytes,
  generationReady,
  maxGeneratedSourceLines = 0,
  maxComponentNodeCount,
  stateIndexExternal,
}) {
  const estimatedTokens = Math.ceil(implementationBytes / 3.5);
  const failures = [
    !generationReady && 'generation evidence is incomplete',
    estimatedTokens > 2500 && `implementation entrypoint exceeds 2500 tokens (${estimatedTokens})`,
    maxComponentNodeCount > 120 &&
      `readable component shard exceeds 120 nodes (${maxComponentNodeCount})`,
    maxGeneratedSourceLines > 200 &&
      `generated React source exceeds 200 lines (${maxGeneratedSourceLines})`,
    !stateIndexExternal && 'interaction states are embedded instead of externally indexed',
  ].filter(Boolean);
  return {
    schemaVersion: 1,
    purpose: 'Fast agent-consumption readiness for native implementation.',
    ready: failures.length === 0,
    failures,
    implementationBytes,
    estimatedTokens,
    maxGeneratedSourceLines,
    maxComponentNodeCount,
    stateIndexExternal,
  };
}
