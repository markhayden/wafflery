import { isAbsolute } from "node:path";

export type PipelineOptions = {
  start: boolean;
  noOpen: boolean;
  /** A persistent library directory, kept between runs; a fresh temporary one when absent. */
  library?: string;
  /** Register the tools in the library and exit, for a caller that runs the host itself. */
  registerOnly: boolean;
};

export function pipelineOptions(flags: string[]): PipelineOptions {
  const options: PipelineOptions = { start: false, noOpen: false, registerOnly: false };
  const seen = new Set<string>();
  for (let index = 0; index < flags.length; index += 1) {
    const flag = flags[index]!;
    if (seen.has(flag)) throw new Error(`${flag} given more than once`);
    seen.add(flag);
    if (flag === "--start") options.start = true;
    else if (flag === "--no-open") options.noOpen = true;
    else if (flag === "--register-only") options.registerOnly = true;
    else if (flag === "--library") {
      const value = flags[++index];
      if (!value || !isAbsolute(value)) throw new Error("--library needs an absolute directory");
      options.library = value;
    } else throw new Error("Expected only --start, --no-open, --library <dir> and --register-only");
  }
  if (options.registerOnly && (options.start || options.noOpen))
    throw new Error("--register-only starts nothing, so it takes neither --start nor --no-open");
  return options;
}
