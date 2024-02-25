TODO:

update the tc-base branch with:
- [*] the notes from the stash
- [*] add the updated logic from ci-workflow-test.sh to ci_manager_test.yml
- [*] push tc-base
- [*] rebase all 77 branches on top of it and push them remotely
- [*] merge tc-base into main and push main
- then create the PR
- raise the PR
- [*] conduct the experiments and populate the table

the actual PR should keep the removed (for the purpose of dummifying) steps and `${{ github.base_ref }}`


++### Notes
++
++Please, note the following:
++- The script intentionally (for the purpose of its own readability as well as not concealing possibly not very meaningful comment lines) doesn't handle lines involving multi-line comment symbols, optionally followed by whitespace and followed by single-line comment symbols, i.e., `/* ... */ // comment`.
++  - This is a case which one shouldn't expect to see in a codebase regardless.
++- The script ignores changes to `*.rs` file lines containing only whitespace, i.e., those are treated - in terms of deciding whether to run tests or not - as if those were comment-only lines.
++- The script is able to handle comments involving multi-line comment symbols, when they are utilised on a single line.
++  - For a more elaborate treatment of multi-line comments, perhaps some sort of static code analysis should be employed instead.
++- The `*.rs` proof-of-concept can be arbitrarily extended to other file types, such as `*.roc`, as per [#6276](https://github.com/roc-lang/roc/issues/6276).

run-test-case() {
    if [ $(run-ignored-check "${1}") = "y" ]; then
        if [ $(run-rs-check "${1}") = "y" ]; then
            echo "full"
            return
        fi
    fi
    echo "none"
}

run-ignored-check() {
    if git diff --name-only workflow-tc-base "${1}" | grep -qvE '(\.md$|\.css$|\.html$|^AUTHORS$)'; then
        echo "y"
        return
    fi
    echo "n"
}

run-rs-check() {
    if git diff --name-only workflow-tc-base "${1}" | grep -qvE '(\.md$|\.css$|\.html$|^AUTHORS$|\.rs)'; then
        echo "y"
        return
    fi
    if git diff --unified=0 workflow-tc-base "${1}" '../*.rs' | grep -E --color=never '^[+-]' | grep -qvE '^(\+\+\+|\-\-\-|[+-]\s*($|\/\/|[+-]|\/\*.*\*\/\s*$))'; then
        echo "y"
        return
    fi
    echo "n"
}