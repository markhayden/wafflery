export function pipelineOptions(flags: string[]) {
  if (
    new Set(flags).size !== flags.length ||
    flags.some((flag) => flag !== "--start" && flag !== "--no-open")
  ) {
    throw new Error("Expected only --start and/or --no-open, each at most once");
  }
  return { start: flags.includes("--start"), noOpen: flags.includes("--no-open") };
}
