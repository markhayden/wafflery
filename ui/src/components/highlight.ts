/** Tokens for a CodeBlock: JSON or a shell command, split into kinds a stylesheet can colour.
 * The text of the tokens, joined, is always exactly the input, so copying and search see the
 * same characters. No HTML is built from strings; CodeBlock renders each token as a span. */
export type Token = { text: string; kind?: "key" | "string" | "number" | "literal" | "punct" | "command" | "flag" | "path" };
export type Language = "json" | "shell" | "text";

const JSON_TOKEN = /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|\b(true|false|null)\b|([{}[\],])/g;

export function highlight(text: string, language: Language): Token[] {
  if (language === "json") return tokenize(text, JSON_TOKEN, (match) =>
    match[1] !== undefined ? [{ text: match[1], kind: match[2] ? "key" : "string" }, ...(match[2] ? [{ text: match[2], kind: "punct" as const }] : [])]
      : match[3] !== undefined ? [{ text: match[3], kind: "number" }]
      : match[4] !== undefined ? [{ text: match[4], kind: "literal" }]
      : [{ text: match[5]!, kind: "punct" }]);
  if (language === "shell") {
    // A word is classed by what it holds, quoted or not: a flag, a path, or the program at the
    // start of the line or after a connector (&&, ||, |, ;).
    let expectCommand = true;
    return tokenize(text, /(&&|\|\||[|;])|('[^']*'|"(?:[^"\\]|\\.)*"|[^\s'"&|;]+)/g, (match) => {
      if (match[1] !== undefined) { expectCommand = true; return [{ text: match[1], kind: "punct" }]; }
      const word = match[2]!, bare = /^['"]/.test(word) ? word.slice(1, -1) : word;
      const kind: Token["kind"] = expectCommand ? "command" : /^--?[A-Za-z]/.test(bare) ? "flag" : /^(~|\.{0,2}\/|\/)/.test(bare) ? "path" : "string";
      expectCommand = false;
      return [{ text: word, kind }];
    });
  }
  return [{ text }];
}

function tokenize(text: string, pattern: RegExp, make: (match: RegExpExecArray) => Token[]): Token[] {
  const tokens: Token[] = [];
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > last) tokens.push({ text: text.slice(last, match.index) });
    tokens.push(...make(match as RegExpExecArray));
    last = match.index + match[0].length;
  }
  if (last < text.length) tokens.push({ text: text.slice(last) });
  return tokens;
}
