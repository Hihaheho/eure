// Tests for Eure string keys in sections (including delimited literal strings)
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

test('string keys: literal strings as section keys', async (t: TestContext) => {
  const results = await tokenize([
    "@ '#'",
    "@ <'What's \"query?\"'>",
    "@ '#'.'Introduction'"
  ])

  printTokens(results)

  // All lines should recognize strings as valid section keys
  for (let i = 0; i < results.length; i++) {
    const tokens = results[i].tokens
    const hasAt = tokens.some(t => t.scopes.includes('punctuation.definition.section.eure'))
    const hasString = tokens.some(t =>
      t.scopes.includes('entity.name.section.eure') ||
      t.scopes.some(s => s.startsWith('string'))
    )

    t.assert.strictEqual(hasAt, true, `Line ${i}: @ should be section punctuation`)
    t.assert.strictEqual(hasString, true, `Line ${i}: String should be recognized as section key`)
  }
})
