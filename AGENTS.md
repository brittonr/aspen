# Agent Guidelines

## Bug Fixes

When fixing a bug or error, **always write a test that reproduces it first**, then fix the code to make the test pass. Don't just fix the bug — prevent it from coming back.

1. **Reproduce**: Write a failing test that triggers the exact bug or error.
2. **Fix**: Apply the minimal code change to resolve the issue.
3. **Verify**: Confirm the new test passes along with all existing tests.
