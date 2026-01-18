// Comprehensive tests for binding patterns
// Tests all combinations of key types × binding types × value types
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

// Helper to get text from token
function getText(line: string, tok: { startIndex: number; endIndex: number }): string {
  return line.substring(tok.startIndex, tok.endIndex)
}

// Helper to find token with specific scope and text
function hasTokenWithScopeAndText(
  line: string,
  tokens: { startIndex: number; endIndex: number; scopes: string[] }[],
  scope: string,
  text?: string
): boolean {
  return tokens.some(tok =>
    tok.scopes.includes(scope) && (text === undefined || getText(line, tok) === text)
  )
}

// =============================================================================
// KEY TYPES
// =============================================================================

test('binding keys: identifier', async (t: TestContext) => {
  const results = await tokenize(['name = "value"'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'variable.other.property.eure', 'name'),
    'identifier key should be variable.other.property'
  )
})

test('binding keys: double-quoted string', async (t: TestContext) => {
  const results = await tokenize(['"my-key" = "value"'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'string.quoted.double.eure', 'my-key'),
    'double-quoted key should be string.quoted.double'
  )
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'keyword.operator.assignment.eure'),
    'should have assignment operator'
  )
})

test('binding keys: single-quoted string', async (t: TestContext) => {
  const results = await tokenize(["'my-key' = \"value\""])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'string.quoted.single.eure', 'my-key'),
    'single-quoted key should be string.quoted.single'
  )
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'keyword.operator.assignment.eure'),
    'should have assignment operator'
  )
})

test('binding keys: delimited literal string <\'...\'>', async (t: TestContext) => {
  const results = await tokenize(["<'my-key'> = \"value\""])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(
    tokens.some(tok => tok.scopes.includes('string.quoted.single.eure')),
    'delimited string key should be string.quoted.single'
  )
})

test('binding keys: extension $key', async (t: TestContext) => {
  const results = await tokenize(['$variant = "union"'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'punctuation.definition.extension.eure', '$'),
    'should have extension punctuation'
  )
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'entity.name.tag.eure', 'variant'),
    'extension name should be entity.name.tag'
  )
})

test('binding keys: array index key[]', async (t: TestContext) => {
  const results = await tokenize(['items[] = "value"'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'variable.other.property.eure', 'items'),
    'should have identifier'
  )
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'punctuation.definition.array.eure', '[]'),
    'should have array index'
  )
})

test('binding keys: dotted path', async (t: TestContext) => {
  const results = await tokenize(['a.b.c = 1'])
  printTokens(results)

  const { tokens } = results[0]
  const dots = tokens.filter(tok => tok.scopes.includes('punctuation.separator.eure'))
  t.assert.strictEqual(dots.length, 2, 'should have 2 dot separators')
})

test('binding keys: complex path with mixed types', async (t: TestContext) => {
  const results = await tokenize(["$ext.'key'.items[] = 1"])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(tokens.some(tok => tok.scopes.includes('entity.name.tag.eure')), 'has extension')
  t.assert.ok(tokens.some(tok => tok.scopes.includes('string.quoted.single.eure')), 'has string key')
  t.assert.ok(tokens.some(tok => tok.scopes.includes('punctuation.definition.array.eure')), 'has array index')
})

// =============================================================================
// BINDING TYPES
// =============================================================================

test('binding type: value binding (=)', async (t: TestContext) => {
  const results = await tokenize(['key = "value"'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'keyword.operator.assignment.eure', '='),
    '= should be assignment operator'
  )
})

test('binding type: text binding (:)', async (t: TestContext) => {
  const results = await tokenize(['key: some text here'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'punctuation.definition.text.eure', ':'),
    ': should be text punctuation'
  )
  t.assert.ok(
    tokens.some(tok => tok.scopes.includes('string.unquoted.text.eure')),
    'text should be string.unquoted.text'
  )
})

test('binding type: section binding ({)', async (t: TestContext) => {
  const results = await tokenize(['key { inner = 1 }'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'variable.other.property.eure', 'key'),
    'key before { should be property'
  )
})

// =============================================================================
// VALUE TYPES (after =)
// =============================================================================

test('value type: double-quoted string', async (t: TestContext) => {
  const results = await tokenize(['key = "hello"'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'string.quoted.double.eure', 'hello'),
    'value should be string.quoted.double'
  )
})

test('value type: single-quoted string', async (t: TestContext) => {
  const results = await tokenize(["key = 'hello'"])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'string.quoted.single.eure', 'hello'),
    'value should be string.quoted.single'
  )
})

test('value type: integer', async (t: TestContext) => {
  const results = await tokenize(['key = 42'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'constant.numeric.integer.eure', '42'),
    'value should be constant.numeric.integer'
  )
})

test('value type: float', async (t: TestContext) => {
  const results = await tokenize(['key = 3.14'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'constant.numeric.float.eure', '3.14'),
    'value should be constant.numeric.float'
  )
})

test('value type: boolean', async (t: TestContext) => {
  const results = await tokenize(['a = true', 'b = false'])
  printTokens(results)

  t.assert.ok(
    hasTokenWithScopeAndText(results[0].line, results[0].tokens, 'constant.language.boolean.eure', 'true'),
    'true should be boolean'
  )
  t.assert.ok(
    hasTokenWithScopeAndText(results[1].line, results[1].tokens, 'constant.language.boolean.eure', 'false'),
    'false should be boolean'
  )
})

test('value type: null', async (t: TestContext) => {
  const results = await tokenize(['key = null'])
  printTokens(results)

  const { line, tokens } = results[0]
  t.assert.ok(
    hasTokenWithScopeAndText(line, tokens, 'constant.language.null.eure', 'null'),
    'null should be constant.language.null'
  )
})

test('value type: array', async (t: TestContext) => {
  const results = await tokenize(['key = [1, 2, 3]'])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.array.begin.eure')
  ))
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.array.end.eure')
  ))
})

test('value type: object', async (t: TestContext) => {
  const results = await tokenize(['key = { a = 1 }'])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.object.begin.eure')
  ))
})

test('value type: tuple', async (t: TestContext) => {
  const results = await tokenize(['key = (1, 2)'])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.tuple.begin.eure')
  ))
})

test('value type: code block', async (t: TestContext) => {
  const results = await tokenize([
    'key = ```markdown',
    'content',
    '```'
  ])
  printTokens(results)

  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.code.begin.eure')
  ))
  t.assert.ok(results[2].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.code.end.eure')
  ))
})

test('value type: inline code', async (t: TestContext) => {
  const results = await tokenize(['key = `inline`'])
  printTokens(results)

  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('markup.inline.raw.eure')
  ))
})

// =============================================================================
// EDGE CASES - These are the fragile cases
// =============================================================================

test('edge case: single-quoted key with text binding', async (t: TestContext) => {
  const results = await tokenize(["'key': text value"])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  ), ': after string key should be text binding')
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('string.unquoted.text.eure')
  ), 'text after : should be unquoted text')
})

test('edge case: single-quoted key with value binding', async (t: TestContext) => {
  const results = await tokenize(["'key' = 123"])
  printTokens(results)

  const { tokens } = results[0]
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('keyword.operator.assignment.eure')
  ), '= after string key should be assignment')
  t.assert.ok(tokens.some(tok =>
    tok.scopes.includes('constant.numeric.integer.eure')
  ), 'number value should be recognized')
})

test('edge case: apostrophe in text binding', async (t: TestContext) => {
  const results = await tokenize([
    "title: What's this?",
    "next = 1"
  ])
  printTokens(results)

  // Line 0: should have text binding with apostrophe inside
  const line0 = results[0]
  const textTokens = line0.tokens.filter(tok =>
    tok.scopes.includes('string.unquoted.text.eure')
  )
  const fullText = textTokens.map(tok => getText(line0.line, tok)).join('')
  t.assert.ok(fullText.includes("What's"), "apostrophe in text should be part of unquoted text")

  // Line 1: should be a normal binding
  t.assert.ok(results[1].tokens.some(tok =>
    tok.scopes.includes('keyword.operator.assignment.eure')
  ), 'next line should have assignment operator')
})

test('edge case: string key with apostrophe in text', async (t: TestContext) => {
  const results = await tokenize([
    "'###': What's query system?",
    "next = 1"
  ])
  printTokens(results)

  // The colon should be text binding
  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  ), ': should be text binding')

  // The text should include the apostrophe
  const textTokens = results[0].tokens.filter(tok =>
    tok.scopes.includes('string.unquoted.text.eure')
  )
  t.assert.ok(textTokens.length > 0, 'should have text tokens')

  // Next line should NOT be consumed by unclosed string
  t.assert.ok(!results[1].tokens.some(tok =>
    tok.scopes.includes('string.quoted.single.eure')
  ), 'next line should not be inside a string')
})

test('edge case: array index followed by code block', async (t: TestContext) => {
  const results = await tokenize([
    'items[] = ```markdown',
    'content',
    '```'
  ])
  printTokens(results)

  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.array.eure')
  ), 'should have array index')
  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.code.begin.eure')
  ), 'should have code block begin')
})

test('edge case: consecutive bindings', async (t: TestContext) => {
  const results = await tokenize([
    'a = 1',
    'b = "two"',
    "c = 'three'",
    'd = true'
  ])
  printTokens(results)

  // Each line should have its own binding
  for (let i = 0; i < 4; i++) {
    t.assert.ok(results[i].tokens.some(tok =>
      tok.scopes.includes('keyword.operator.assignment.eure')
    ), `line ${i} should have assignment`)
  }
})

test('edge case: binding after single-quoted value', async (t: TestContext) => {
  const results = await tokenize([
    "a = 'value'",
    "b = 2"
  ])
  printTokens(results)

  // Line 1 should be a normal binding, not consumed by line 0
  const line1 = results[1]
  t.assert.ok(
    hasTokenWithScopeAndText(line1.line, line1.tokens, 'variable.other.property.eure', 'b'),
    'b should be a property'
  )
  t.assert.ok(line1.tokens.some(tok =>
    tok.scopes.includes('constant.numeric.integer.eure')
  ), '2 should be a number')
})

test('edge case: nested object with string keys', async (t: TestContext) => {
  const results = await tokenize([
    "outer {",
    "  'inner': text with apostrophe's",
    "  next = 1",
    "}"
  ])
  printTokens(results)

  // Line 1: string key with text binding
  t.assert.ok(results[1].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  ), 'should have text binding')

  // Line 2: should be normal binding
  t.assert.ok(results[2].tokens.some(tok =>
    tok.scopes.includes('keyword.operator.assignment.eure')
  ), 'should have assignment')
})

test('edge case: double-quoted key with text binding', async (t: TestContext) => {
  const results = await tokenize(['"key": text value'])
  printTokens(results)

  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  ), ': after double-quoted key should be text binding')
})

test('edge case: multiple string keys in path', async (t: TestContext) => {
  const results = await tokenize(["'a'.'b'.'c' = 1"])
  printTokens(results)

  const stringTokens = results[0].tokens.filter(tok =>
    tok.scopes.includes('string.quoted.single.eure')
  )
  t.assert.ok(stringTokens.length >= 3, 'should have multiple string segments')
  t.assert.ok(results[0].tokens.some(tok =>
    tok.scopes.includes('keyword.operator.assignment.eure')
  ), 'should have assignment')
})

// =============================================================================
// REGRESSION TESTS - Specific bugs that were fixed
// =============================================================================

test('regression: string key followed by array index and code block', async (t: TestContext) => {
  // This was the original bug report
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

  // Line 1: should have text binding
  t.assert.ok(results[1].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.text.eure')
  ), 'line 1: should have text binding')

  // Line 3: should have code block, NOT string
  t.assert.ok(!results[3].tokens.some(tok =>
    tok.scopes.includes('string.quoted.single.eure')
  ), 'line 3: should NOT be inside a string')
  t.assert.ok(results[3].tokens.some(tok =>
    tok.scopes.includes('punctuation.definition.code.begin.eure')
  ), 'line 3: should have code block begin')
})
