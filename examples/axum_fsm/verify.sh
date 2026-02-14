#!/bin/bash
set -e

# Create an order
echo "Creating order..."
curl -X POST -H "Content-Type: application/json" -d '{"id": "order-123", "items": ["item1", "item2"], "total": 100}' http://localhost:3000/orders
echo ""

# Get initial status (Created)
echo "Status:"
curl http://localhost:3000/orders/order-123
echo ""

# Validate
echo "Validating..."
curl -X POST http://localhost:3000/orders/order-123/validate
echo ""
sleep 0.2

# Get status (Validated)
echo "Status:"
curl http://localhost:3000/orders/order-123
echo ""

# Charge
echo "Charging..."
curl -X POST http://localhost:3000/orders/order-123/charge
echo ""
sleep 0.3

# Get status (Charged)
echo "Status:"
curl http://localhost:3000/orders/order-123
echo ""

# Ship
echo "Shipping..."
curl -X POST http://localhost:3000/orders/order-123/ship
echo ""
sleep 0.4

# Get final status (Shipped)
echo "Status:"
curl http://localhost:3000/orders/order-123
echo ""
