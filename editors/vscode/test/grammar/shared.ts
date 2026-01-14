// Shared test utilities for TextMate grammar tests
import * as fs from 'node:fs'
import * as path from 'node:path'
import vsctm from 'vscode-textmate'
import onig from 'vscode-oniguruma'

const { Registry, INITIAL, parseRawGrammar } = vsctm
const { loadWASM, OnigScanner, OnigString } = onig

// Re-export types and constants for tests
export type IToken = vsctm.IToken
export { INITIAL }

// Dummy grammars for embedded languages (for testing)
export const DUMMY_GRAMMARS: Record<string, string> = {
  'source.rust': 'test/fixtures/grammars/rust.json',
  'source.json': 'test/fixtures/grammars/json.json',
  'source.yaml': 'test/fixtures/grammars/yaml.json',
  'source.toml': 'test/fixtures/grammars/toml.json',
  'source.js': 'test/fixtures/grammars/javascript.json',
  'source.ts': 'test/fixtures/grammars/typescript.json',
  'source.python': 'test/fixtures/grammars/python.json',
  'source.css': 'test/fixtures/grammars/css.json',
  'source.sql': 'test/fixtures/grammars/sql.json',
  'source.shell': 'test/fixtures/grammars/shell.json',
  'text.html.basic': 'test/fixtures/grammars/html.json',
  'text.html.markdown': 'test/fixtures/grammars/markdown.json',
}

// Load oniguruma WASM
const wasmBin = fs.readFileSync(
  path.join(import.meta.dirname, '../../node_modules/vscode-oniguruma/release/onig.wasm')
).buffer

const vscodeOnigurumaLib = loadWASM(wasmBin).then(() => ({
  createOnigScanner: (patterns: string[]) => new OnigScanner(patterns),
  createOnigString: (s: string) => new OnigString(s)
}))

// Create and return a configured grammar registry
export function createRegistry() {
  return new Registry({
    onigLib: vscodeOnigurumaLib,
    loadGrammar: async (scopeName: string) => {
      if (scopeName === 'source.eure') {
        const grammarPath = path.join(import.meta.dirname, '../../syntaxes/eure.tmLanguage.json')
        const content = fs.readFileSync(grammarPath, 'utf8')
        return parseRawGrammar(content, grammarPath)
      }

      // Load dummy grammar for embedded languages
      const dummyGrammarPath = DUMMY_GRAMMARS[scopeName]
      if (dummyGrammarPath !== undefined) {
        const grammarPath = path.join(import.meta.dirname, '../..', dummyGrammarPath)
        const content = fs.readFileSync(grammarPath, 'utf8')
        return parseRawGrammar(content, grammarPath)
      }

      return null
    }
  })
}

// Helper to tokenize multiple lines
export async function tokenize(lines: string[]) {
  const registry = createRegistry()
  const grammar = await registry.loadGrammar('source.eure')
  if (!grammar) throw new Error('Failed to load grammar')

  let ruleStack = INITIAL
  const results: { line: string; tokens: IToken[] }[] = []

  for (const line of lines) {
    const lineTokens = grammar.tokenizeLine(line, ruleStack)
    results.push({ line, tokens: lineTokens.tokens })
    ruleStack = lineTokens.ruleStack
  }
  return results
}

// Helper to print tokens for debugging
export function printTokens(results: { line: string; tokens: IToken[] }[]) {
  for (let i = 0; i < results.length; i++) {
    const result = results[i]
    if (result === undefined) continue
    const { line, tokens } = result
    console.log(`\nLine ${i}: "${line}"`)
    for (const token of tokens) {
      const text = line.substring(token.startIndex, token.endIndex)
      console.log(`  [${token.startIndex}-${token.endIndex}] "${text}" => ${token.scopes.join(', ')}`)
    }
  }
}
