# Category Encoders

The Category Encoders pack catches target leakage into test rows, silent misconfiguration, runtime errors on category_encoders 2.11, and removed or deprecated forms on files that import `category_encoders`. Rules fire only on calls bound to `category_encoders` imports.

## Leakage

- transform-with-test-target
- encode-before-split

## Codes that are wrong without an error

- ordinal-mapping-pairs
- ordinal-mapping-unknown
- zero-regularization
- hashing-default-components
- cols-left-to-dtype
- nan-into-pipeline

## Code that raises

- supervised-encoder-without-target
- invalid-handle-option
- named-cols-after-array-step
- wrapper-fit-returns-none

## Removed and deprecated forms

- removed-in-category-encoders
- deprecated-in-category-encoders

## Encoders that scikit-learn ships

- sklearn-ships-encoder

## Reproducible results

- encoder-noise-without-seed
