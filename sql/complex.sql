WITH recent_orders AS (
    SELECT customer_id, order_date, total_amount
    FROM orders 
    WHERE order_date >= '2023-01-01'
),
customer_stats AS (
    SELECT 
        c.customer_id,
        c.name,
        COUNT(ro.customer_id) as order_count,
        SUM(ro.total_amount) as total_spent
    FROM customers c
    LEFT JOIN recent_orders ro ON c.customer_id = ro.customer_id
    GROUP BY c.customer_id, c.name
)
SELECT 
    cs.name,
    cs.order_count,
    cs.total_spent,
    p.product_name
FROM customer_stats cs
JOIN order_items oi ON cs.customer_id = oi.customer_id
JOIN products p ON oi.product_id = p.product_id
WHERE cs.total_spent > (
    SELECT AVG(total_spent) 
    FROM customer_stats
)
ORDER BY cs.total_spent DESC;
