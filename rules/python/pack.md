# Python

The Python rules report style choices that no formatter or type checker makes, and they run on
every file, whatever it imports. The `python-best-practices` skill in
[`skills/`](https://github.com/bjoernaagaard/xpd/tree/main/skills) holds the rest of the style,
and the `python-review` skill reviews code against it.

| Group | Rules |
| --- | --- |
| Names | `module-private-name`, `vague-module-name` |
| Modules | `import-side-effect` |
| Types | `typing-cast`, `dict-str-any`, `record-without-namedtuple`, `json-without-type`, `same-type-conversion`, `float-around-number` |
| Functions | `pass-through-function`, `multi-line-call-argument` |
| Tests | `mock-import`, `test-skip`, `test-rerun`, `test-sleep` |
| Comprehensions | `comprehension-merges-dicts`, `unpacked-comprehension`, `comprehension-two-fors`, `comprehension-multi-line`, `comprehension-among-arguments` |
| Standard library | `setdefault-without-defaultdict`, `counting-loop-without-counter`, `strptime-without-fromisoformat`, `elapsed-without-perf-counter`, `toml-without-tomllib` |
| Speed and memory | `front-insert-in-loop`, `orjson-loads-decoded`, `slotted-base-without-slots` |

## Names

Name each module after what it holds, and leave the underscore off module level names.

- module-private-name
- vague-module-name

## Modules

Keep calls and environment reads out of the top level of a module, so an import runs no work.

- import-side-effect

## Types

State the real type in a signature, so the type checker can check the keys and the values the function uses.

- typing-cast
- dict-str-any
- record-without-namedtuple
- json-without-type
- same-type-conversion
- float-around-number

## Functions

Call a function directly when a second function would only pass its parameters on.

- pass-through-function
- multi-line-call-argument

## Tests

Test with real objects and let every test run once, because a mock, a skip, a rerun or a sleep lets a test pass while the code is wrong.

- mock-import
- test-skip
- test-rerun
- test-sleep

## Comprehensions

Write a loop when a comprehension merges dicts, unpacks, holds two `for` clauses or runs over several lines, and give a comprehension a name before you pass it with other arguments, because both are easier to read.

- comprehension-merges-dicts
- unpacked-comprehension
- comprehension-two-fors
- comprehension-multi-line
- comprehension-among-arguments

## Standard library

Use what the standard library already has before you write it by hand.

- setdefault-without-defaultdict
- counting-loop-without-counter
- strptime-without-fromisoformat
- elapsed-without-perf-counter
- toml-without-tomllib

## Speed and memory

Leave out work that the measured alternative skips, when the code can always take the
alternative.

- front-insert-in-loop
- orjson-loads-decoded
- slotted-base-without-slots
