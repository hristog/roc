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
        else
            if [ $(run-roc-check "${1}") = "y" ]; then
                echo "full"
            else
                echo "none"
            fi
        fi
    else
        echo "none"
    fi
}

run-ignored-check() {
    if git diff --name-only workflow-tc-base "${1}" | grep -qvE '(\.md$|\.css$|\.html$|^AUTHORS$)'; then
        echo "y"
    else
        echo "n"
    fi
}

run-rs-check() {
    if git diff --name-only workflow-tc-base "${1}" | grep -qvE '(\.md$|\.css$|\.html$|^AUTHORS$|\.rs|\.roc)'; then
        echo "y"
    else
        if git diff --unified=0 workflow-tc-base "${1}" '../*.rs' | grep -E --color=never '^[+-]' | grep -qvE '^(\+\+\+|\-\-\-|[+-]\s*($|\/\/|\/\*.*\*\/\s*$))'; then
            echo "y"
        else
            echo "n"
        fi
    fi
}

run-roc-check() {
    if git diff --name-only workflow-tc-base "${1}" | grep -qvE '(\.md$|\.css$|\.html$|^AUTHORS$|\.rs|\.roc)'; then
        echo "y"
    else
        if git diff --unified=0 workflow-tc-base "${1}" '../*.roc' | grep -E --color=never '^[+-]' | grep -qvE '^(\+\+\+|\-\-\-|[+-]\s*($|#))'; then
            echo "y"
        else
            echo "n"
        fi
    fi
}

update-all-tc-branches() {
    original_dir=$(pwd)
    original_branch=$(git branch | grep \* | cut -d' ' -f2)

    for test_case in $(cat ./test_cases.expected); do
        cd ${original_dir}/..
        branch_name=$(echo ${test_case} | cut -d, -f1)
        git checkout ${branch_name}
        git rebase main
        git rebase workflow-tc-base
    done
    git checkout ${original_branch}
    cd ${original_dir}
}

push-all-tc-branches-to-origin() {
    original_dir=$(pwd)
    original_branch=$(git branch | grep \* | cut -d' ' -f2)

    for test_case in $(cat ./test_cases.expected); do
        cd ${original_dir}/..
        branch_name=$(echo ${test_case} | cut -d, -f1)
        git checkout ${branch_name}
        git push -u --force origin ${branch_name}
    done
    git checkout ${original_branch}
    cd ${original_dir}
}

"$@"
