#!/usr/bin/env python3
# -*- encoding: utf-8 -*-
# vim: set et ts=4 sts=4 sw=4

import os
import re
import subprocess
import sys


if __name__ == '__main__':
    # Skip this check if instructed
    if re.search(r'skip.?changelog', os.environ['CI_COMMIT_MESSAGE'], flags=re.I) is not None:
        print('Changelog skipped.')
        sys.exit(0)

    treeish = 'main..'
    if os.environ['CI_PIPELINE_SOURCE'] == 'push' and os.environ['CI_COMMIT_REF_NAME'] == 'main':
        treeish = '{}..'.format(os.environ['CI_COMMIT_BEFORE_SHA'])
    if os.environ['CI_PIPELINE_SOURCE'] == 'merge_request_event':
        treeish = '{}..'.format(os.environ['CI_MERGE_REQUEST_DIFF_BASE_SHA'])

    diff=subprocess.run(
        ['git', 'diff-tree', '--no-commit-id', '-r', treeish, '--'],
        capture_output=True,
    )

    if diff.stderr:
        print(f'An error occured: {diff.stderr}')
        sys.exit(1)

    print(f'Files in {treeish}:\n{diff.stdout}')

    # Verify if at least a single changelog file has been added or edited
    if re.search(rb'[AM]\s+\S*changelog', diff.stdout, flags=re.I) is None:
        print(
            'Please update the Changelog. Use "skip-changelog" '
            'in the commit message to skip this check.',
        )
        sys.exit(1)

    print('Changelog has been updated as expected.')
