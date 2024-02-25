#!/bin/bash

non_ignored=b934662fc
ignored_commit=9c43df3f8

while true; do
    branch_name=$(./ci-workflow-test.sh run-all-test-cases | head -1 | cut -d' ' -f3)
    if [ -z "${branch_name}" ]; then break; fi
    #echo "${branch_name}" | grep -qw "ignored-and-non-ignored-files-changed" && break
    git checkout ${branch_name}
    merge_branch=${branch_name%-ignored-and-non-ignored-files-changed}
    git merge ${merge_branch}
    git cherry-pick ${non_ignored}
    git cherry-pick ${ignored_commit}
    git push -u  origin ${branch_name}
done
