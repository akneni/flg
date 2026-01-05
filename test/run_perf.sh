#!/bin/bash
# Script to compile the test program, run it with perf, and output perf.data
# Generates both on-CPU and off-CPU flamegraphs

set -e

rm -f test/perf_oncpu.data test/perf_offcpu.data

# Compile with debug info for better symbols
echo "Compiling cpu_test.c..."
gcc -O2 -g -fno-omit-frame-pointer -o test/cpu_test test/cpu_test.c -lm

# Run with perf record for on-CPU time (cpu-clock sampling)
echo "Running perf record for on-CPU profiling..."
perf record -g --call-graph dwarf -o test/perf_oncpu.data ./test/cpu_test 2000

# Run with perf record for off-CPU time (sched:sched_switch tracepoint)
# Requires root for scheduler tracepoints
echo "Running perf record for off-CPU profiling..."
sudo perf record -g -e sched:sched_switch -o test/perf_offcpu.data ./test/cpu_test 2000
sudo chown $(whoami):$(whoami) test/perf_offcpu.data

# Generate flamegraphs
echo "Generating on-CPU flamegraph..."
cargo run -- gen test/perf_oncpu.data -o flamegraph_oncpu.html

echo "Generating off-CPU flamegraph..."
cargo run -- gen test/perf_offcpu.data -o flamegraph_offcpu.html

cargo run -- gen test/*.data -o flamegraph.html

echo "Done! Generated flamegraph_oncpu.html and flamegraph_offcpu.html"
