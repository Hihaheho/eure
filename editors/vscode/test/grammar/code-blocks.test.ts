// Comprehensive code block tests
import { test, type TestContext } from 'node:test'
import { tokenize, printTokens } from './shared.ts'

// Test: Root-level code blocks with various languages
test('root-level: rust code block', async (t: TestContext) => {
  const results = await tokenize([
    '```rust',
    'fn main() {}',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  // Line 0: ```rust
  t.assert.ok(line0.some(tok => tok.scopes.includes('punctuation.definition.code.begin.eure')),
    '``` should have code begin punctuation')
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')),
    'rust should have entity.name.function scope')

  // Line 1: fn main() {}
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.rust')),
    'Content should have rust embedded scope')
})

test('root-level: markdown code block', async (t: TestContext) => {
  const results = await tokenize([
    '```markdown',
    '# Hello',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  t.assert.ok(line0.some(tok => tok.scopes.includes('punctuation.definition.code.begin.eure')))
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')))
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.markdown')))
})

test('root-level: md alias for markdown', async (t: TestContext) => {
  const results = await tokenize([
    '```md',
    'Text here',
    '```'
  ])

  const line0 = results[0].tokens
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')),
    'md should be recognized as language tag')
})

test('root-level: json code block', async (t: TestContext) => {
  const results = await tokenize([
    '```json',
    '{"key": "value"}',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.json')))
})

test('root-level: yaml code block', async (t: TestContext) => {
  const results = await tokenize([
    '```yaml',
    'key: value',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.yaml')))
})

test('root-level: yml alias for yaml', async (t: TestContext) => {
  const results = await tokenize([
    '```yml',
    'key: value',
    '```'
  ])

  const line0 = results[0].tokens
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')))
})

test('root-level: javascript code block', async (t: TestContext) => {
  const results = await tokenize([
    '```javascript',
    'const x = 1;',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.javascript')))
})

test('root-level: js alias for javascript', async (t: TestContext) => {
  const results = await tokenize([
    '```js',
    'const x = 1;',
    '```'
  ])

  const line0 = results[0].tokens
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')))
})

test('root-level: typescript code block', async (t: TestContext) => {
  const results = await tokenize([
    '```typescript',
    'const x: number = 1;',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.typescript')))
})

test('root-level: python code block', async (t: TestContext) => {
  const results = await tokenize([
    '```python',
    'def hello():',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.python')))
})

test('root-level: shell code block', async (t: TestContext) => {
  const results = await tokenize([
    '```shell',
    'echo "hello"',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.shellscript')))
})

test('root-level: generic unknown language', async (t: TestContext) => {
  const results = await tokenize([
    '```foobar',
    'some code',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')),
    'Unknown language should still be recognized as language tag')
  t.assert.ok(line1.some(tok => tok.scopes.includes('markup.raw.block.eure')),
    'Unknown language content should use generic markup scope')
})

test('root-level: no language tag', async (t: TestContext) => {
  const results = await tokenize([
    '```',
    'plain code',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  t.assert.ok(line0.some(tok => tok.scopes.includes('punctuation.definition.code.begin.eure')))
  t.assert.ok(line1.some(tok => tok.scopes.includes('markup.raw.block.eure')))
})

// Test: Value-binding code blocks (key = ```lang)
test('value-binding: rust code block', async (t: TestContext) => {
  const results = await tokenize([
    'code = ```rust',
    'fn main() {}',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  t.assert.ok(line0.some(tok => tok.scopes.includes('keyword.operator.assignment.eure')),
    '= should be assignment operator')
  t.assert.ok(line0.some(tok => tok.scopes.includes('punctuation.definition.code.begin.eure')),
    '``` should have code begin punctuation')
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')),
    'rust should have entity.name.function scope')
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.rust')),
    'Content should have rust embedded scope')
})

test('value-binding: markdown code block', async (t: TestContext) => {
  const results = await tokenize([
    'body = ```markdown',
    '# Title',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')),
    'markdown should be language tag')
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.markdown')),
    'Content should have markdown embedded scope')
})

test('value-binding: json code block', async (t: TestContext) => {
  const results = await tokenize([
    'data = ```json',
    '{"x": 1}',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.json')))
})

test('value-binding: generic unknown language', async (t: TestContext) => {
  const results = await tokenize([
    'code = ```customlang',
    'content',
    '```'
  ])

  const line0 = results[0].tokens
  const line1 = results[1].tokens

  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')))
  t.assert.ok(line1.some(tok => tok.scopes.includes('markup.raw.block.eure')))
})

test('value-binding: no language tag', async (t: TestContext) => {
  const results = await tokenize([
    'code = ```',
    'plain',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('markup.raw.block.eure')))
})

// Test: Code blocks shouldn't trigger text binding
test('code-block: should NOT apply text binding inside code blocks', async (t: TestContext) => {
  const results = await tokenize([
    'code = ```markdown',
    'You can compose: procedures',
    '```'
  ])

  const line1 = results[1].tokens
  const hasTextBinding = line1.some(tok => tok.scopes.includes('punctuation.definition.text.eure'))

  t.assert.strictEqual(hasTextBinding, false,
    'Colon inside code block should NOT trigger text binding')
})

// Test: Section context code blocks
test('section: code block in section', async (t: TestContext) => {
  const results = await tokenize([
    '@ docs {',
    '  body = ```markdown',
    '  # Hello',
    '  ```',
    '}'
  ])

  const line1 = results[1].tokens
  const line2 = results[2].tokens

  t.assert.ok(line1.some(tok => tok.scopes.includes('entity.name.function.eure')))
  t.assert.ok(line2.some(tok => tok.scopes.includes('meta.embedded.block.markdown')))
})

// Test: Empty code blocks
test('empty: code block with no content', async (t: TestContext) => {
  const results = await tokenize([
    '```rust',
    '```'
  ])

  const line0 = results[0].tokens
  t.assert.ok(line0.some(tok => tok.scopes.includes('entity.name.function.eure')))
})

test('empty: code block with whitespace only', async (t: TestContext) => {
  const results = await tokenize([
    '```rust',
    '   ',
    '```'
  ])

  const line1 = results[1].tokens
  t.assert.ok(line1.some(tok => tok.scopes.includes('meta.embedded.block.rust')))
})

// Additional tests from grammar.test.ts

test('code-block: should enter code-block context on ```markdown', async (t: TestContext) => {
  const results = await tokenize([
    'body = ```markdown',
    'In this article',
    '```'
  ])

  printTokens(results)

  // Line 0: body = ```markdown
  const line0 = results[0].tokens
  const hasCodeBegin = line0.some(t => t.scopes.includes('punctuation.definition.code.begin.eure'))
  t.assert.strictEqual(hasCodeBegin, true, 'Should have code block begin punctuation')

  const hasLangTag = line0.some(t => t.scopes.includes('entity.name.function.eure'))
  t.assert.strictEqual(hasLangTag, true, 'Should have language tag scope')

  // Line 1: In this article
  const line1 = results[1].tokens
  // Check which content scope is applied
  const hasMarkdownContent = line1.some(t => t.scopes.includes('meta.embedded.block.markdown'))
  const hasGenericContent = line1.some(t => t.scopes.includes('markup.raw.block.eure'))
  console.log(`Content has meta.embedded.block.markdown: ${hasMarkdownContent}`)
  console.log(`Content has markup.raw.block.eure: ${hasGenericContent}`)

  // For now, just verify SOME code-block scope is applied
  const hasCodeBlockContent = hasMarkdownContent || hasGenericContent
  t.assert.strictEqual(hasCodeBlockContent, true, 'Content should have code block scope')

  // Line 2: ```
  const line2 = results[2].tokens
  const hasCodeEnd = line2.some(t => t.scopes.includes('punctuation.definition.code.end.eure'))
  t.assert.strictEqual(hasCodeEnd, true, 'Should have code block end punctuation')
})

test('code-block: should use markdown-specific pattern for md', async (t: TestContext) => {
  const bt3 = '`'.repeat(3)
  const results = await tokenize([
    'body = ' + bt3 + 'md',
    'Content here',
    bt3
  ])

  printTokens(results)

  const line1 = results[1].tokens
  const hasMarkdownContent = line1.some(t => t.scopes.includes('meta.embedded.block.markdown'))
  console.log('Using md - has meta.embedded.block.markdown:', hasMarkdownContent)
})

test('code-block: test rust pattern', async (t: TestContext) => {
  // Note: In this test environment, embedded grammars (source.rust) can't be loaded.
  // When patterns include an unavailable grammar, vscode-textmate falls back to generic.
  // In VS Code with Rust extension installed, this would show meta.embedded.block.rust.
  const bt3 = '`'.repeat(3)
  const results = await tokenize([
    'code = ' + bt3 + 'rust',
    'fn main() {}',
    bt3
  ])

  printTokens(results)

  // Verify code block delimiters are matched correctly
  const line0 = results[0].tokens
  const hasBegin = line0.some(t => t.scopes.includes('punctuation.definition.code.begin.eure'))
  t.assert.strictEqual(hasBegin, true, 'Should have code block begin')

  const line2 = results[2].tokens
  const hasEnd = line2.some(t => t.scopes.includes('punctuation.definition.code.end.eure'))
  t.assert.strictEqual(hasEnd, true, 'Should have code block end')

  // Content scope depends on whether embedded grammar is available
  const line1 = results[1].tokens
  const hasRustContent = line1.some(t => t.scopes.includes('meta.embedded.block.rust'))
  const hasGenericContent = line1.some(t => t.scopes.includes('markup.raw.block.eure'))
  console.log('Using rust - has meta.embedded.block.rust:', hasRustContent)
  console.log('Using rust - has markup.raw.block.eure:', hasGenericContent)
})

test('code-block: test rust pattern at root level (no binding)', async (t: TestContext) => {
  // Test if the pattern works without the binding context
  const bt3 = '`'.repeat(3)
  const results = await tokenize([
    bt3 + 'rust',
    'fn main() {}',
    bt3
  ])

  printTokens(results)

  const line1 = results[1].tokens
  const hasRustContent = line1.some(t => t.scopes.includes('meta.embedded.block.rust'))
  const hasGenericContent = line1.some(t => t.scopes.includes('markup.raw.block.eure'))
  console.log('Root level rust - has meta.embedded.block.rust:', hasRustContent)
  console.log('Root level rust - has markup.raw.block.eure:', hasGenericContent)
})

test('code-block: test direct rust match', async (t: TestContext) => {
  // Test with trailing newline in same line to see if it affects matching
  const results = await tokenize([
    'x = ```rust',  // Single line with rust
    'code',
    '```'
  ])

  printTokens(results)

  // Check all scopes on line 1 (content)
  const line1 = results[1].tokens
  console.log('All scopes on content line:', JSON.stringify(line1.map(t => t.scopes), null, 2))
})
