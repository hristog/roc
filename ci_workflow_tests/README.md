### Instructions

To run all test-cases, locally, in a way which simulates the remote GitHub workflow:

```
chmod u+x ci-workflow-test.sh
./ci-workflow-test.sh run-all-test-cases
```

Sample output:
```
77 tests ran: 77 ok, 0 failed.
```

### Notes

Please, note the following:
- The script intentionally (for the purpose of its own readability as well as not concealing possibly not very meaningful comment lines) doesn't handle lines involving multi-line comment symbols, optionally followed by whitespace and followed by single-line comment symbols, i.e., `/* ... */ // comment`.
  - This is a case which one shouldn't expect to see in a codebase regardless.
- The script ignores changes to `*.rs` file lines containing only whitespace, i.e., those are treated - in terms of deciding whether to run tests or not - as if those were comment-only lines.
- The script is able to handle comments involving multi-line comment symbols, when they are utilised on a single line.
  - For a more elaborate treatment of multi-line comments, perhaps some sort of static code analysis should be employed instead.
- The `*.rs` proof-of-concept can be arbitrarily extended to other file types, such as `*.roc`, as per [#6276](https://github.com/roc-lang/roc/issues/6276).
