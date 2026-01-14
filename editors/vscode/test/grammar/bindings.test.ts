// Tests for Eure binding syntax (= and :)
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

test('simple binding: key = value should work', async (t: TestContext) => {
  const results = await tokenize([
    'name = "hello"',
    'count = 42'
  ])

  printTokens(results)

  // Line 0: name = "hello"
  const line0 = results[0].tokens
  const hasString = line0.some(t => t.scopes.some(s => s.includes('string')))
  t.assert.strictEqual(hasString, true, 'Should have string scope')

  // Line 1: count = 42
  const line1 = results[1].tokens
  const hasNumber = line1.some(t => t.scopes.some(s => s.includes('constant.numeric')))
  t.assert.strictEqual(hasNumber, true, 'Should have number scope')
})

test('text binding: key: text should work', async (t: TestContext) => {
  const results = await tokenize([
    'description: This is some text'
  ])

  printTokens(results)

  const line0 = results[0].tokens
  const hasTextBinding = line0.some(t => t.scopes.includes('string.unquoted.text.eure'))
  t.assert.strictEqual(hasTextBinding, true, 'Should have text binding scope')
})

test('code-block: should NOT apply text binding inside code blocks', async (t: TestContext) => {
  const results = await tokenize([
    'code = ```markdown',
    'You can compose: procedures',
    '```'
  ])

  printTokens(results)

  const line1 = results[1].tokens

  // The `:` should NOT trigger string.unquoted.text.eure
  const hasTextBinding = line1.some(t => t.scopes.includes('string.unquoted.text.eure'))
  t.assert.strictEqual(hasTextBinding, false, 'Text binding should not apply inside code blocks')
})
