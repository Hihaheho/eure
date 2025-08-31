# Variant Validation Issues Update

## Summary
After applying simple fixes, we've improved from 7/18 to 16/18 tests passing.

## Fixes Applied

### 1. Test Syntax Corrections
- Fixed `$variant: value` to `$variant = "value"` in object literals (EURE syntax requirement)
- Fixed grammar ordering: moved `$variant-repr` before variant sections (EURE grammar rule)

### 2. Untagged Variant Detection
- Modified validator to not report `VariantDiscriminatorMissing` for untagged variants
- Simplified untagged variant matching to accept first variant with required fields

### 3. Adjacently Tagged Missing Content Field
- Added validation to report `RequiredFieldMissing` when content field is absent
- Fixed proper error reporting for adjacently tagged variants

### 4. Internally Tagged Invalid Tag Value
- Added check for tag fields with invalid variant values
- Now properly reports `UnknownVariant` instead of `VariantDiscriminatorMissing`

### 5. Untagged Variant No Match Error
- Fixed error reporting when no untagged variant matches
- Now properly reports `UnknownVariant` with "no matching variant" message

## Remaining Complex Issues

### 1. Variant Cascade Type Interaction
**Problem**: Cascade types within variants report unexpected fields
**Root Cause**: Variant context not properly propagated through cascade type validation
**Complexity**: Requires careful context management across type boundaries

### 2. Complex Array Types in Variants
**Problem**: Arrays within variants get type mismatch (expected object, got array)
**Root Cause**: Variant validation assumes object content, doesn't handle array variants
**Complexity**: Fundamental assumption in variant validation needs revision


## Architectural Issues

### 1. Variant Context Management
The `variant_context` and `variant_repr_context` HashMaps don't properly propagate through nested validations, causing issues with:
- Cascade types losing variant context
- Nested variants not inheriting parent context
- Tag field exclusion not working in all cases

### 2. Untagged Variant Validation Strategy
Current approach:
- Lightweight field checking
- Returns first variant with required fields
- Doesn't actually validate structure

Better approach would be:
- Try full validation for each variant
- Cache results to avoid redundant work
- Report most specific error if all fail

### 3. Error Reporting Granularity
Current issues:
- Generic `VariantDiscriminatorMissing` for multiple failure modes
- No indication of which variants were tried
- Missing context about why variants didn't match

## Recommended Next Steps

### Short Term (Quick Fixes)
1. ~~Fix adjacently tagged content field validation~~ ✅
2. ~~Add proper error for invalid tag values~~ ✅
3. ~~Improve untagged "no match" error reporting~~ ✅

### Medium Term (Structural Changes)
1. Refactor variant context propagation
2. Implement proper untagged variant validation with caching
3. Support array-based variants

### Long Term (Architecture)
1. Redesign variant validation to be more composable
2. Add variant validation hints/debugging
3. Implement variant inference for better error messages

## Test Status After Fixes

| Test | Status | Issue |
|------|--------|-------|
| test_tagged_variant_basic | ✅ | - |
| test_tagged_variant_with_extension | ✅ | - |
| test_tagged_variant_multiple_keys_error | ✅ | - |
| test_empty_variant | ✅ | - |
| test_internally_tagged_variant_basic | ✅ | - |
| test_internally_tagged_missing_tag | ✅ | - |
| test_internally_tagged_invalid_tag_value | ✅ | - |
| test_adjacently_tagged_variant_basic | ✅ | - |
| test_adjacently_tagged_missing_content | ✅ | - |
| test_untagged_variant_basic | ✅ | - |
| test_untagged_variant_ambiguous | ✅ | - |
| test_untagged_variant_no_match | ✅ | - |
| test_variant_field_type_mismatch | ✅ | - |
| test_variant_required_field_missing | ✅ | - |
| test_variant_unexpected_fields | ✅ | - |
| test_variant_cascade_type_interaction | ❌ | Context propagation |
| test_variant_with_complex_types | ❌ | Array variants |
| test_deeply_nested_variants | ✅ | - |