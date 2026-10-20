# Core architecture

Core implementations are grouped by language. Python source code and tests live
together under `core/python/`, with functional areas beneath that directory.
Future languages may receive sibling directories with independent dependencies.
The project deliberately avoids component → implementations → language nesting.
