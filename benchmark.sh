#!/bin/bash
set -euf -o pipefail

R3BAR="./target/release/r3bar"

perf record -g $R3BAR -b 15
perf script | stackcollapse-perf | flamegraph > flame.svg
