#!/usr/bin/env bash
# Linux CI checks. Run from any directory; this script moves to the repository root.
# Exit 0: every executed check passed, including git diff --check.
# Exit 3: web checks passed, but there is no Git repository.
#         git diff --check was not run and is not a pass.

set -eu
export CI=true

cd "$(dirname "$0")/.."

echo "CHECK yarn-install"
corepack yarn install --immutable

echo "CHECK web-build"
corepack yarn workspace @datagrizzly/web build

echo "CHECK web-check"
corepack yarn workspace @datagrizzly/web check

echo "CHECK env-classification"
node scripts/check-m0-env.mjs

echo "CHECK content-fixtures"
node scripts/check-m0-content.mjs

echo "CHECK layout-home"
node scripts/check-m1-layout.mjs

echo "CHECK project-pages"
node scripts/check-m1-projects.mjs

echo "CHECK research-pages"
node scripts/check-m1-research.mjs

echo "CHECK seo-pages"
node scripts/check-m1-seo.mjs

if [ ! -d .git ]; then
  echo "GIT_CHECK=DEFERRED"
  echo "No Git repository. S03 and S04 remain deferred, so git diff --check was not run and is not a pass."
  exit 3
fi

echo "CHECK git-diff-check"
git diff --check
echo "CHECK_OK"
