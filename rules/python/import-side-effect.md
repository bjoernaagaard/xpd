A call at module level runs whenever the module is imported. Move it into a function that `main()` or the caller runs.

```diff lang="python"
 import logging
 import os
 
 import dspy
 
-logging.basicConfig(level=logging.INFO)
-dspy.configure(lm=dspy.LM("openai/gpt-4o-mini"))
-REGION = os.environ.get("REGION", "eu")
-
 
 def answer(question: str) -> str:
     return dspy.Predict("question -> answer")(question=question).answer
+
+
+def main() -> None:
+    logging.basicConfig(level=logging.INFO)
+    dspy.configure(lm=dspy.LM("openai/gpt-4o-mini"))
+    print(answer(os.environ.get("QUESTION", "What is xpd?")))
```

## What the rule reports

The rule reports a call that stands alone at module level, and a read or write of `os.environ` or
`os.getenv` at module level.

The rule leaves alone the cases in the list:

## How to fix it

xpd has no automatic fix, so make the change by hand.

A module level call runs on the first import, in an order nobody chose, and again in every
process that imports the module. A program that imports the module to use one function gets
its logging and its library settings changed, and the environment is read before the program
has parsed its own arguments.

- A file that is run and never imported
  - A file with no function and no class. It is a script, which runs from top to bottom.
  - A file with a `# %%` cell marker. An editor such as VS Code runs it cell by cell in an
    interactive window.
  - A `__main__.py`.
- A call that sets up the module itself
  - A method that edits a collection the module defines, such as `NAMES.append("id")`.
  - A call whose name starts with `register`, and `include_router`, `add_typer`,
    `add_middleware` and `mount` on a web or command line app.
  - `pytest.importorskip`, which is how a test module skips itself.
- A call inside `if`, `try`, `with` or a class body, and a call whose result is assigned.
