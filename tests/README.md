# Tests

Implementation tests under `profile_management/implementations/python/` exercise
Python-specific units and failure paths. Contract tests are black-box and must
not import production implementation modules; `profile_management/runner.py`
adapts scenarios to the current CLI. Run all tests with `make test`, or focus on
one implementation with `python3 -m unittest discover -s tests/profile_management/implementations/python -t . -v`.
