#!/bin/sh
# $1 = omp stdout capture. Pass iff the answer points at the right file
# (line number tolerated — the file is the signal).
grep -q 'src/memory.rs' "$1"
