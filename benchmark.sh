#!/bin/bash
set -euf -o pipefail

# http://blog.adamperry.me/rust/2016/07/24/profiling-rust-perf-flamegraph/

R3BAR="./target/release/r3bar"
DELAY=0
REC_TIME=60

perf record --delay $DELAY -g $R3BAR -b $REC_TIME
perf script | stackcollapse-perf | flamegraph > flame.svg
perf stat record --delay $DELAY -o perf.stat.data $R3BAR -b $REC_TIME
