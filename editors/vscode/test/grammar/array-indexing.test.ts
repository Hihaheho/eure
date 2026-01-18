// Tests for array indexing syntax with code blocks
// Repro cases for bug: `key[] = ```markdown` tokenized as string.quoted.single.eure
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

// REPRO: Bug where c[] = ```markdown is tokenized as string.quoted.single.eure
// Root cause: After single-quoted key like '###', the colon is not recognized as
// text-binding operator. Then apostrophe in text (What's) starts an unclosed string.
test('array-indexing: REPRO - single-quoted key with apostrophe in text value', async (t: TestContext) => {
  const results = await tokenize([
    "query-system {",
    "  '###': What's query system?",
    "",
    "  c[] = ```markdown",
    "  Content here",
    "  ```",
    "}"
  ])

  printTokens(results)

  // Line 1: '###': What's query system?
  // BUG: The colon after '###' should trigger text-binding
  const line1 = results[1].tokens
  const colonIsTextBinding = line1.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  )
  t.assert.strictEqual(colonIsTextBinding, true,
    "Colon after single-quoted key should be text-binding punctuation")

  // The text after colon should be unquoted text, NOT a new binding key
  const hasUnquotedText = line1.some(tok =>
    tok.scopes.includes('string.unquoted.text.eure')
  )
  t.assert.strictEqual(hasUnquotedText, true,
    "Text after colon should be string.unquoted.text.eure")

  // Line 3: c[] = ```markdown
  // BUG: This line is incorrectly tokenized as string.quoted.single.eure
  const line3 = results[3].tokens
  const hasWrongStringScope = line3.some(tok =>
    tok.scopes.includes('string.quoted.single.eure')
  )
  t.assert.strictEqual(hasWrongStringScope, false,
    'c[] = ```markdown should NOT have string.quoted.single.eure scope')

  const hasCodeBegin = line3.some(tok =>
    tok.scopes.includes('punctuation.definition.code.begin.eure')
  )
  t.assert.strictEqual(hasCodeBegin, true,
    'Should have code block begin punctuation')
})

// Minimal repro: single-quoted key followed by text-binding with apostrophe
test('array-indexing: REPRO - minimal case', async (t: TestContext) => {
  const results = await tokenize([
    "'key': What's this?",
    "next = 1"
  ])

  printTokens(results)

  // Line 0: the colon should be text-binding
  const line0 = results[0].tokens
  const colonIsTextBinding = line0.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  )
  t.assert.strictEqual(colonIsTextBinding, true,
    "Colon after 'key' should be text-binding")

  // Line 1: should NOT be consumed by unclosed string
  const line1 = results[1].tokens
  const hasWrongStringScope = line1.some(tok =>
    tok.scopes.includes('string.quoted.single.eure')
  )
  t.assert.strictEqual(hasWrongStringScope, false,
    "Line after text binding should not be inside a string")

  const hasAssignment = line1.some(tok =>
    tok.scopes.includes('keyword.operator.assignment.eure')
  )
  t.assert.strictEqual(hasAssignment, true,
    "Should recognize = as assignment operator")
})

// Verify basic array indexing works when NOT preceded by problematic context
test('array-indexing: basic key[] with code block (no preceding string key)', async (t: TestContext) => {
  const results = await tokenize([
    'c[] = ```markdown',
    '# Content',
    '```'
  ])

  const line0 = results[0].tokens
  const hasCodeBegin = line0.some(tok =>
    tok.scopes.includes('punctuation.definition.code.begin.eure')
  )
  t.assert.strictEqual(hasCodeBegin, true, 'Should have code block begin')
})
