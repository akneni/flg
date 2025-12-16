#!/bin/bash
# Script to compile the test program, run it with perf, and output perf.data

set -e

rm test/perf.data

# Compile with debug info for better symbols
echo "Compiling cpu_test.c..."
gcc -O2 -g -fno-omit-frame-pointer -o test/cpu_test test/cpu_test.c -lm

# Run with perf record
echo "Running perf record..."
perf record -g --call-graph dwarf -o test/perf.data ./test/cpu_test 2000

cargo run -- gen test/perf.data
