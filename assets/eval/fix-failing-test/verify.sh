#!/bin/sh
# $1 = omp stdout capture (unused). Pass iff the test suite is green.
cargo test 2>&1 | grep -q 'test result: ok'
