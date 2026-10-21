# Python core

Python source code and tests are grouped together under this directory.
Functional areas live beneath it; future languages may use sibling directories
with independent dependencies. This avoids component → implementations →
language nesting.

Run the test suite with `python3 -m unittest discover -s core/python/tests -t core/python -v`.
