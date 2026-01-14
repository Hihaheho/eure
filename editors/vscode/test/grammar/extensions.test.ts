// Tests for Eure extension syntax ($ prefix)
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

test('extensions: various $-prefixed syntax', async (t: TestContext) => {
  const results = await tokenize([
    '$ext = "ext"',
    'key.$ext-key = 2',
    '$types.action {',
    '  $variant: union',
    '}'
  ])

  printTokens(results)

  // Verify all $ symbols get extension punctuation
  for (let i = 0; i < results.length; i++) {
    const tokens = results[i].tokens
    const line = results[i].line

    if (line.includes('$')) {
      const hasDollar = tokens.some(t => t.scopes.includes('punctuation.definition.extension.eure'))
      t.assert.strictEqual(hasDollar, true, `Line ${i} should have extension punctuation for $`)
    }
  }
})
