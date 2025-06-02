#!/bin/bash

# Demo script for kickoff calculator functionality
# This script demonstrates the calculator features implemented in kickoff

echo "=== Kickoff Calculator Demo ==="
echo
echo "The calculator automatically detects mathematical expressions and shows results."
echo "Try typing these expressions in kickoff:"
echo

echo "Basic arithmetic:"
echo "  10-5          → 5"
echo "  2+3*4         → 14"
echo "  (1+2)*3       → 9"
echo "  42            → 42"
echo "  -5            → -5"
echo "  10/2          → 5"
echo

echo "Order of operations:"
echo "  2+3*4         → 14 (not 20)"
echo "  (2+3)*4       → 20"
echo "  10-6/2        → 7 (not 2)"
echo

echo "Decimals:"
echo "  3.5*2         → 7"
echo "  10.5/2.1      → 5"
echo "  3.14*2        → 6.28"
echo

echo "Negative numbers:"
echo "  -5+10         → 5"
echo "  (-2)*3        → -6"
echo "  10-(-5)       → 15"
echo

echo "Complex expressions:"
echo "  ((1+2)*3)/4   → 2.25"
echo "  2*3+4*5       → 26"
echo

echo "With spaces (also works):"
echo "  10 - 5        → 5"
echo "  2 + 3 * 4     → 14"
echo

echo "=== Usage Instructions ==="
echo "1. Launch kickoff"
echo "2. Type any mathematical expression"
echo "3. The result will appear at the top of the results"
echo "4. Press Enter while the calculator result is selected to copy it to clipboard"
echo

echo "Note: The calculator result will only appear for valid mathematical expressions."
echo "Invalid expressions like 'hello', '++', or incomplete expressions won't trigger it."