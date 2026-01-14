// Tests for validating the compiled grammar structure
import { test, type TestContext } from 'node:test'
import * as fs from 'node:fs'
import * as path from 'node:path'

test('grammar: should be valid JSON with expected structure', async (t: TestContext) => {
  const grammarPath = path.join(import.meta.dirname, '../../syntaxes/eure.tmLanguage.json')
  const content = fs.readFileSync(grammarPath, 'utf8')

  // Should parse as valid JSON
  const grammar = JSON.parse(content)

  t.assert.strictEqual(grammar.scopeName, 'source.eure', 'Should have correct scopeName')
  t.assert.ok(grammar.repository, 'Should have repository')

  // Check code-block patterns exist
  t.assert.ok(grammar.repository['code-block'], 'Should have code-block pattern')
  t.assert.ok(grammar.repository['code-block-3-rust'], 'Should have rust pattern')
  t.assert.ok(grammar.repository['code-block-3-generic'], 'Should have generic pattern')

  // Check rust pattern structure
  const rustPattern = grammar.repository['code-block-3-rust']
  t.assert.ok(rustPattern.begin, 'Rust pattern should have begin')
  t.assert.ok(rustPattern.end, 'Rust pattern should have end')
  t.assert.ok(rustPattern.contentName || rustPattern.name, 'Rust pattern should have contentName or name')

  // Check generic pattern structure
  const genericPattern = grammar.repository['code-block-3-generic']
  t.assert.ok(genericPattern.begin, 'Generic pattern should have begin')
  t.assert.ok(genericPattern.end, 'Generic pattern should have end')

  // Check that rust comes before generic in code-block includes
  const codeBlockPatterns = grammar.repository['code-block'].patterns
  const rustIndex = codeBlockPatterns.findIndex((p: {include?: string}) => p.include === '#code-block-3-rust')
  const genericIndex = codeBlockPatterns.findIndex((p: {include?: string}) => p.include === '#code-block-3-generic')

  t.assert.ok(rustIndex >= 0, 'code-block should include rust pattern')
  t.assert.ok(genericIndex >= 0, 'code-block should include generic pattern')
  t.assert.ok(rustIndex < genericIndex, 'Rust pattern should come before generic pattern')

  console.log('Grammar structure validated')
  console.log('  rust begin:', rustPattern.begin)
  console.log('  generic begin:', genericPattern.begin)
  console.log('  rust index in code-block:', rustIndex)
  console.log('  generic index in code-block:', genericIndex)
})
