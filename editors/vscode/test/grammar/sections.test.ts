// Tests for Eure section syntax (@ prefix)
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

test('section: with extension like @ $frontmatter', async (t: TestContext) => {
  const results = await tokenize([
    '@ $frontmatter',
    'title: Query System Wins'
  ])

  printTokens(results)

  // Verify line 0: @ $frontmatter
  const line0 = results[0].tokens
  const hasAt = line0.some(t => t.scopes.includes('punctuation.definition.section.eure'))
  const hasDollar = line0.some(t => t.scopes.includes('punctuation.definition.extension.eure'))
  const hasExtName = line0.some(t => t.scopes.includes('entity.name.tag.eure'))

  t.assert.strictEqual(hasAt, true, '@ should have section punctuation')
  t.assert.strictEqual(hasDollar, true, '$ should have extension punctuation')
  t.assert.strictEqual(hasExtName, true, 'frontmatter should have extension name scope')

  // Verify line 1: title: Query System Wins
  const line1 = results[1].tokens
  const hasTextColon = line1.some(t => t.scopes.includes('punctuation.definition.text.eure'))
  const hasTextContent = line1.some(t => t.scopes.includes('string.unquoted.text.eure'))

  t.assert.strictEqual(hasTextColon, true, ': should have text binding punctuation')
  t.assert.strictEqual(hasTextContent, true, 'Text should have string.unquoted scope')
})

test('section: string key with nested sections', async (t: TestContext) => {
  const results = await tokenize([
    '@ "# Introduction" {',
    '  @ "## What\'s query system?" {',
    '    = ```markdown',
    '    In this article',
    '    ```',
    '  }',
    '}'
  ])

  printTokens(results)

  // Verify both @ symbols have same scope
  const line0 = results[0].tokens
  const line1 = results[1].tokens

  const hasAt0 = line0.some(t => t.scopes.includes('punctuation.definition.section.eure'))
  const hasAt1 = line1.some(t => t.scopes.includes('punctuation.definition.section.eure'))

  t.assert.strictEqual(hasAt0, true, 'First @ should have section scope')
  t.assert.strictEqual(hasAt1, true, 'Second @ should have section scope')

  // Verify string is entity.name.section.eure
  const hasString0 = line0.some(t =>
    t.scopes.includes('entity.name.section.eure') ||
    t.scopes.includes('string.quoted.double.eure')
  )
  t.assert.strictEqual(hasString0, true, 'String key should be recognized as section name')

  // Verify code block inside nested section
  const line2 = results[2].tokens
  const hasCodeBegin = line2.some(t => t.scopes.includes('punctuation.definition.code.begin.eure'))
  t.assert.strictEqual(hasCodeBegin, true, 'Code block should work inside nested sections')

  // Verify : in string doesn't trigger text binding
  const line3 = results[3].tokens
  const hasWrongTextBinding = line3.some(t =>
    t.scopes.includes('punctuation.definition.text.eure') &&
    !t.scopes.includes('meta.embedded.block.markdown') &&
    !t.scopes.includes('markup.raw.block.eure')
  )
  t.assert.strictEqual(hasWrongTextBinding, false, ': inside code block should not be text binding')
})

test('section: code block with = assignment', async (t: TestContext) => {
  const results = await tokenize([
    '@ "# Why migrate?" {',
    '  body = ```markdown',
    '  Content here',
    '  ```',
    '}'
  ])

  printTokens(results)

  // Line 1: body = ```markdown
  const line1 = results[1].tokens

  const hasKey = line1.some(t => t.scopes.includes('variable.other.property.eure'))
  const hasEquals = line1.some(t => t.scopes.includes('keyword.operator.assignment.eure'))
  const hasCodeBegin = line1.some(t => t.scopes.includes('punctuation.definition.code.begin.eure'))
  const hasLang = line1.some(t => t.scopes.includes('entity.name.function.eure'))

  t.assert.strictEqual(hasKey, true, 'body should be property')
  t.assert.strictEqual(hasEquals, true, '= should be assignment')
  t.assert.strictEqual(hasCodeBegin, true, '``` should start code block')
  t.assert.strictEqual(hasLang, true, 'markdown should be language tag')
})

test('section: with array index syntax', async (t: TestContext) => {
  const results = await tokenize([
    '@ actions[]',
    '$variant: set-text',
    'text: Welcome message'
  ])

  printTokens(results)

  // Line 0: @ actions[]
  const line0 = results[0].tokens
  const hasAt = line0.some(t => t.scopes.includes('punctuation.definition.section.eure'))
  const hasSectionName = line0.some(t => t.scopes.includes('entity.name.section.eure'))
  const hasArrayMarker = line0.some(t => t.scopes.some(s => s.includes('punctuation.definition.array')))

  t.assert.strictEqual(hasAt, true, '@ should be section punctuation')
  t.assert.strictEqual(hasSectionName, true, 'actions should be section name')
  t.assert.strictEqual(hasArrayMarker, true, '[] should be array marker')

  // Line 1: $variant: set-text
  const line1 = results[1].tokens
  const hasDollar = line1.some(t => t.scopes.includes('punctuation.definition.extension.eure'))
  const hasVariant = line1.some(t => t.scopes.includes('entity.name.tag.eure'))

  t.assert.strictEqual(hasDollar, true, '$ should be extension punctuation')
  t.assert.strictEqual(hasVariant, true, 'variant should have extension name')
})

test('section: deeply nested with mixed content', async (t: TestContext) => {
  const results = await tokenize([
    '@ jobs.build {',
    '  runs-on: ubuntu-latest',
    '  @ steps[]',
    '  uses: actions/checkout@v6',
    '  with {',
    '    path = ```',
    '    ~/.cargo/bin/',
    '    ```',
    '  }',
    '}'
  ])

  printTokens(results)

  // Just verify it doesn't crash and key patterns work
  const line0 = results[0].tokens
  const hasAt = line0.some(t => t.scopes.includes('punctuation.definition.section.eure'))
  t.assert.strictEqual(hasAt, true, 'Should parse nested sections')

  const line5 = results[5].tokens
  const hasCodeStart = line5.some(t => t.scopes.includes('punctuation.definition.code.begin.eure'))
  t.assert.strictEqual(hasCodeStart, true, 'Code block should work in deep nesting')
})
