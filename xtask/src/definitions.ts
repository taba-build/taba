/**
 * Checks where go to definition takes each name in `xtask/generate/conformance/usage.ts`. Each
 * `// ^` under a name says where, one comment per definition.
 *
 * Run it through `cargo xtask lint`.
 */

import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, relative } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  createMessageConnection,
  StreamMessageReader,
  StreamMessageWriter,
} from "vscode-jsonrpc/node";

const REPO = fileURLToPath(new URL("../..", import.meta.url));
const USAGE = fileURLToPath(new URL("../generate/conformance/usage.ts", import.meta.url));

/** Where a definition is, as the language server sends it to a client without `linkSupport`. */
type Location = { uri: string; range: { start: { line: number; character: number } } };

function bail(message: string): never {
  console.error(message);
  process.exit(1);
}

/** Each name a `// ^` in `lines` points at, with its definitions as written, in order. */
function expectations(lines: string[]) {
  const comments: { line: number; character: number; definition: string }[] = [];
  let above = -1;
  for (const [line, text] of lines.entries()) {
    if (/^\s*\/\/\s*\^ /.test(text)) {
      const character = text.indexOf("^");
      comments.push({ line: above, character, definition: text.slice(character + 2) });
    } else {
      above = line;
    }
  }
  return [...Map.groupBy(comments, (comment) => comment.line)].flatMap(([line, onLine]) =>
    [...Map.groupBy(onLine, (comment) => comment.character)].map(([character, atName]) => ({
      position: { line, character },
      definitions: atName.map((comment) => comment.definition),
    })),
  );
}

/** A definition as a `// ^` writes it. */
function written({ uri, range: { start } }: Location): string {
  const file = fileURLToPath(uri);
  const text = readFileSync(file, "utf8").split("\n")[start.line] ?? "";
  const name = /^[\w$]*/.exec(text.slice(start.character))?.[0] ?? "";
  const marked = `${text.slice(0, start.character)}[${name}]`;
  return `${relative(dirname(USAGE), file)} ${marked.trimStart()}`;
}

// xtask's own `tsc`, which pnpm puts on a script's PATH
const server = spawn("tsc", ["--lsp", "--stdio"]);
let stderr = "";
server.stderr.on("data", (chunk: Buffer) => (stderr += chunk));
const early = (code: number | null, signal: NodeJS.Signals | null) =>
  bail(`tsc --lsp exited with ${code ?? signal}:\n${stderr}`);
server.on("exit", early);

const connection = createMessageConnection(
  new StreamMessageReader(server.stdout),
  new StreamMessageWriter(server.stdin),
);
connection.listen();
const rootUri = pathToFileURL(dirname(USAGE)).href;
await connection.sendRequest("initialize", { processId: process.pid, rootUri, capabilities: {} });
await connection.sendNotification("initialized", {});

const usage = readFileSync(USAGE, "utf8");
const uri = pathToFileURL(USAGE).href;
const textDocument = { uri, languageId: "typescript", version: 1, text: usage };
await connection.sendNotification("textDocument/didOpen", { textDocument });

const mismatches: string[] = [];
for (const { position, definitions } of expectations(usage.split("\n"))) {
  const params = { textDocument: { uri }, position };
  const result = await connection.sendRequest<Location[] | null>("textDocument/definition", params);
  const actual = (result ?? []).map(written).join("\n  ");
  const expected = definitions.join("\n  ");
  if (actual !== expected) {
    const at = `${relative(REPO, USAGE)}:${position.line + 1}:${position.character + 1}`;
    mismatches.push(`${at} is defined at\n  ${actual || "nowhere"}\nnot\n  ${expected}`);
  }
}

await connection.sendRequest("shutdown");
server.off("exit", early);
await connection.sendNotification("exit");
if (mismatches.length > 0) bail(mismatches.join("\n\n"));
