#!/bin/bash

# Configuration
CONCURRENCY=50
TOTAL_ORDERS=5000000
API_BASE="http://localhost:3000"

echo "🚀 Starting stress test: $TOTAL_ORDERS orders with $CONCURRENCY parallel workers"

# Function to drive a single order
drive_order() {
    local i=$1
    local id="order-$i"
    
    # 1. Create
    curl -s -o /dev/null -X POST -H "Content-Type: application/json" \
        -d "{\"id\": \"$id\", \"items\": [\"item$i\"], \"total\": $i}" \
        "$API_BASE/orders"
    
    # 2. Validate
    curl -s -o /dev/null -X POST "$API_BASE/orders/$id/validate"
    
    # 3. Charge
    curl -s -o /dev/null -X POST "$API_BASE/orders/$id/charge"
    
    # 4. Ship
    curl -s -o /dev/null -X POST "$API_BASE/orders/$id/ship"
}

export -f drive_order
export API_BASE

# Run in parallel using xargs
seq 1 $TOTAL_ORDERS | xargs -P $CONCURRENCY -I {} bash -c "drive_order {}"

echo "✅ Stress test completed!"
echo "Check your tokio-console or server logs for performance metrics."
