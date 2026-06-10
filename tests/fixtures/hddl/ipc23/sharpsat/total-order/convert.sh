#!/bin/bash

# Counter starting at 1
counter=1

# Loop over CNF files (sorted for consistent ordering)
for cnf_file in cnfs/*.cnf; do
    # Format number with leading zero (pb01, pb02, ...)
    printf -v num "%02d" "$counter"
    output_file="pb${num}.hddl"

    echo "Converting $cnf_file -> $output_file"
    python3 cnf2hddl.py "$cnf_file" > "$output_file"

    ((counter++))
done
