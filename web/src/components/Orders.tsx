import { useEffect, useState } from 'react';
import './Orders.css';

interface Order {
  id: number;
  customer_name: string;
  total_amount: number;
  status: string;
  created_at: string;
}

export function Orders() {
  const [orders, setOrders] = useState<Order[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchOrders = async () => {
      try {
        const response = await fetch('http://localhost:8000/api/orders');
        if (!response.ok) {
          throw new Error('Failed to fetch orders');
        }
        const data = await response.json();
        setOrders(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An error occurred');
      } finally {
        setLoading(false);
      }
    };

    fetchOrders();
  }, []);

  if (loading) return <div>Loading orders...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <div className="orders-container">
      <h2>Orders</h2>
      <div className="orders-grid">
        {orders.map((order) => (
          <div key={order.id} className="order-card">
            <h3>Order #{order.id}</h3>
            <p>Customer: {order.customer_name}</p>
            <p>Amount: ${order.total_amount.toFixed(2)}</p>
            <p>Status: {order.status}</p>
            <p>Created: {new Date(order.created_at).toLocaleString()}</p>
          </div>
        ))}
      </div>
    </div>
  );
} 