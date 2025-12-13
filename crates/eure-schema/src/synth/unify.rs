//! Type unification for synthesized types
//!
//! Unification finds the least upper bound (join) of two types in the type lattice.
//! This is used when synthesizing array types from heterogeneous elements.
//!
//! # Type Lattice
//!
//! ```text
//!                    Any (top)
//!                   / | \
//!           primitives compounds unions
//!                   \ | /
//!                 Never (bottom)
//! ```
//!
//! # Unification Rules
//!
//! | T1 | T2 | unify(T1, T2) |
//! |----|----|----|
//! | T | T | T |
//! | Any | T | T |
//! | Never | T | T |
//! | Hole | T | T |
//! | Int | Text | Int \| Text |
//! | Array<A> | Array<B> | Array<unify(A, B)> |
//! | Tuple(same len) | Tuple | Tuple(elementwise) |
//! | Record | Record (same shape) | Record |
//! | Record | Record (diff shape) | Record \| Record |
//! | otherwise | | Union |

use super::types::{SynthField, SynthRecord, SynthType, SynthUnion};

/// Unify two types into their least upper bound.
///
/// This finds the most specific type that is a supertype of both inputs.
///
/// # Examples
///
/// ```rust,ignore
/// // Same types
/// unify(Integer, Integer) == Integer
///
/// // Different primitives form a union
/// unify(Integer, Text) == Integer | Text
///
/// // Arrays unify element types
/// unify(Array<Int>, Array<Text>) == Array<Int | Text>
///
/// // Holes are absorbed
/// unify(Integer, Hole) == Integer
/// ```
pub fn unify(t1: SynthType, t2: SynthType) -> SynthType {
    // Fast path: identical types
    if t1 == t2 {
        return t1;
    }

    match (t1, t2) {
        // Identity elements
        (SynthType::Any, t) | (t, SynthType::Any) => t,
        (SynthType::Never, t) | (t, SynthType::Never) => t,
        (SynthType::Hole(_), t) | (t, SynthType::Hole(_)) => t,

        // Union flattening
        (SynthType::Union(u1), SynthType::Union(u2)) => {
            let all_variants = u1.variants.into_iter().chain(u2.variants);
            SynthUnion::from_variants(all_variants)
        }
        (SynthType::Union(u), t) | (t, SynthType::Union(u)) => {
            let all_variants = u.variants.into_iter().chain(std::iter::once(t));
            SynthUnion::from_variants(all_variants)
        }

        // Arrays: unify element types
        (SynthType::Array(a), SynthType::Array(b)) => SynthType::Array(Box::new(unify(*a, *b))),

        // Tuples: unify element-wise if same length
        (SynthType::Tuple(a), SynthType::Tuple(b)) if a.len() == b.len() => {
            let unified: Vec<_> = a.into_iter().zip(b).map(|(x, y)| unify(x, y)).collect();
            SynthType::Tuple(unified)
        }

        // Records: check if same shape for potential merging
        (SynthType::Record(r1), SynthType::Record(r2)) => unify_records(r1, r2),

        // Text with different languages
        (SynthType::Text(l1), SynthType::Text(l2)) => {
            // If either is implicit (None), prefer the explicit one
            match (l1, l2) {
                (None, l) | (l, None) => SynthType::Text(l),
                (Some(a), Some(b)) if a == b => SynthType::Text(Some(a)),
                (Some(a), Some(b)) => {
                    // Different explicit languages form a union
                    SynthUnion::from_variants([SynthType::Text(Some(a)), SynthType::Text(Some(b))])
                }
            }
        }

        // Different types: form a union
        (t1, t2) => SynthUnion::from_variants([t1, t2]),
    }
}

/// Unify two records.
///
/// Records with the same field names (regardless of types) are merged by
/// unifying their field types. Records with different shapes form a union.
///
/// This design choice keeps record shapes distinct, which is useful for
/// discriminated unions and type narrowing.
fn unify_records(r1: SynthRecord, r2: SynthRecord) -> SynthType {
    // Check if records have the same field names
    let keys1: std::collections::HashSet<_> = r1.fields.keys().collect();
    let keys2: std::collections::HashSet<_> = r2.fields.keys().collect();

    if keys1 == keys2 {
        // Same shape: merge by unifying field types
        let mut fields = std::collections::HashMap::new();
        for (name, f1) in r1.fields {
            let f2 = r2.fields.get(&name).unwrap();
            fields.insert(
                name,
                SynthField {
                    ty: unify(f1.ty, f2.ty.clone()),
                    optional: f1.optional || f2.optional,
                },
            );
        }
        SynthType::Record(SynthRecord { fields })
    } else {
        // Different shapes: form a union
        SynthUnion::from_variants([SynthType::Record(r1), SynthType::Record(r2)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_same() {
        assert_eq!(
            unify(SynthType::Integer, SynthType::Integer),
            SynthType::Integer
        );
        assert_eq!(
            unify(
                SynthType::Text(Some("rust".into())),
                SynthType::Text(Some("rust".into()))
            ),
            SynthType::Text(Some("rust".into()))
        );
    }

    #[test]
    fn test_unify_any() {
        assert_eq!(
            unify(SynthType::Any, SynthType::Integer),
            SynthType::Integer
        );
        assert_eq!(
            unify(SynthType::Integer, SynthType::Any),
            SynthType::Integer
        );
    }

    #[test]
    fn test_unify_never() {
        assert_eq!(
            unify(SynthType::Never, SynthType::Integer),
            SynthType::Integer
        );
        assert_eq!(
            unify(SynthType::Integer, SynthType::Never),
            SynthType::Integer
        );
    }

    #[test]
    fn test_unify_hole() {
        assert_eq!(
            unify(SynthType::Hole(None), SynthType::Integer),
            SynthType::Integer
        );
        assert_eq!(
            unify(SynthType::Integer, SynthType::Hole(None)),
            SynthType::Integer
        );
    }

    #[test]
    fn test_unify_different_primitives() {
        assert_eq!(
            unify(SynthType::Integer, SynthType::Boolean),
            SynthType::Union(SynthUnion {
                variants: vec![SynthType::Integer, SynthType::Boolean]
            })
        );
    }

    #[test]
    fn test_unify_arrays() {
        let arr1 = SynthType::Array(Box::new(SynthType::Integer));
        let arr2 = SynthType::Array(Box::new(SynthType::Boolean));
        assert_eq!(
            unify(arr1, arr2),
            SynthType::Array(Box::new(SynthType::Union(SynthUnion {
                variants: vec![SynthType::Integer, SynthType::Boolean]
            })))
        );
    }

    #[test]
    fn test_unify_tuples_same_length() {
        let t1 = SynthType::Tuple(vec![SynthType::Integer, SynthType::Boolean]);
        let t2 = SynthType::Tuple(vec![SynthType::Integer, SynthType::Integer]);
        assert_eq!(
            unify(t1, t2),
            SynthType::Tuple(vec![
                SynthType::Integer,
                SynthType::Union(SynthUnion {
                    variants: vec![SynthType::Boolean, SynthType::Integer]
                })
            ])
        );
    }

    #[test]
    fn test_unify_tuples_different_length() {
        let t1 = SynthType::Tuple(vec![SynthType::Integer]);
        let t2 = SynthType::Tuple(vec![SynthType::Integer, SynthType::Boolean]);
        assert_eq!(
            unify(t1.clone(), t2.clone()),
            SynthType::Union(SynthUnion {
                variants: vec![t1, t2]
            })
        );
    }

    #[test]
    fn test_unify_records_same_shape() {
        let r1 = SynthRecord::new([
            ("a".into(), SynthField::required(SynthType::Integer)),
            ("b".into(), SynthField::required(SynthType::Boolean)),
        ]);
        let r2 = SynthRecord::new([
            ("a".into(), SynthField::required(SynthType::Text(None))),
            ("b".into(), SynthField::required(SynthType::Boolean)),
        ]);

        let expected = SynthType::Record(SynthRecord::new([
            (
                "a".into(),
                SynthField::required(SynthType::Union(SynthUnion {
                    variants: vec![SynthType::Integer, SynthType::Text(None)],
                })),
            ),
            ("b".into(), SynthField::required(SynthType::Boolean)),
        ]));
        assert_eq!(
            unify(SynthType::Record(r1), SynthType::Record(r2)),
            expected
        );
    }

    #[test]
    fn test_unify_records_different_shape() {
        let r1 = SynthRecord::new([("a".into(), SynthField::required(SynthType::Integer))]);
        let r2 = SynthRecord::new([
            ("a".into(), SynthField::required(SynthType::Text(None))),
            ("b".into(), SynthField::required(SynthType::Boolean)),
        ]);

        let expected = SynthType::Union(SynthUnion {
            variants: vec![
                SynthType::Record(SynthRecord::new([(
                    "a".into(),
                    SynthField::required(SynthType::Integer),
                )])),
                SynthType::Record(SynthRecord::new([
                    ("a".into(), SynthField::required(SynthType::Text(None))),
                    ("b".into(), SynthField::required(SynthType::Boolean)),
                ])),
            ],
        });
        assert_eq!(
            unify(SynthType::Record(r1), SynthType::Record(r2)),
            expected
        );
    }

    #[test]
    fn test_unify_text_languages() {
        // Implicit + explicit = explicit
        assert_eq!(
            unify(SynthType::Text(None), SynthType::Text(Some("rust".into()))),
            SynthType::Text(Some("rust".into()))
        );

        // Same explicit = same
        assert_eq!(
            unify(
                SynthType::Text(Some("rust".into())),
                SynthType::Text(Some("rust".into()))
            ),
            SynthType::Text(Some("rust".into()))
        );

        // Different explicit = union
        assert_eq!(
            unify(
                SynthType::Text(Some("rust".into())),
                SynthType::Text(Some("python".into())),
            ),
            SynthType::Union(SynthUnion {
                variants: vec![
                    SynthType::Text(Some("rust".into())),
                    SynthType::Text(Some("python".into()))
                ]
            })
        );
    }

    #[test]
    fn test_unify_unions() {
        let u1 = SynthUnion::from_variants([SynthType::Integer, SynthType::Boolean]);
        let u2 = SynthUnion::from_variants([SynthType::Text(None), SynthType::Float]);
        assert_eq!(
            unify(u1, u2),
            SynthType::Union(SynthUnion {
                variants: vec![
                    SynthType::Integer,
                    SynthType::Boolean,
                    SynthType::Text(None),
                    SynthType::Float
                ]
            })
        );
    }

    #[test]
    fn test_unify_union_with_member() {
        let union = SynthUnion::from_variants([SynthType::Integer, SynthType::Boolean]);
        assert_eq!(
            unify(union, SynthType::Text(None)),
            SynthType::Union(SynthUnion {
                variants: vec![
                    SynthType::Integer,
                    SynthType::Boolean,
                    SynthType::Text(None)
                ]
            })
        );
    }
}
