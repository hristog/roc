#!/bin/bash

run-all-test-cases() {
    num_ok=0
    num_failed=0

    for test_case in $(cat ./test_cases.expected); do
        branch_name=$(echo ${test_case} | cut -d, -f1)
        expected=$(echo ${test_case} | cut -d, -f2)

        actual=$(run-test-case ${branch_name})
        if [ "${expected}" != "${actual}" ]; then
            echo "- FAILED: ${branch_name} (expected ${expected}, actual ${actual})"
            let "num_failed=num_failed+1"
        else
            let "num_ok=num_ok+1"
        fi
    done

    let "total=num_ok+num_failed"
    echo "${total} tests ran: ${num_ok} ok, ${num_failed} failed."

    if [ "$num_failed" != "0" ]; then
        return 1
    fi
    return 0
}

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

"$@"
