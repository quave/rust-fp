import { useEffect, useState } from 'react';
import './Transactions.css';
import SearchIcon from '@mui/icons-material/Search';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import CancelIcon from '@mui/icons-material/Cancel';

interface TransactionItem {
  id: number;
  name: string;
  category: string;
  price: number;
}

interface Customer {
  id: number;
  name: string;
  email: string;
}

interface Billing {
  id: number;
  payment_type: string;
  payment_details: string;
  billing_address: string;
}

interface Transaction {
  order: {
    id: number;
    order_number: string;
    delivery_type: string;
    delivery_details: string;
    created_at: string;
    status: 'completed' | 'pending' | 'cancelled';
  };
  items: TransactionItem[];
  customer: Customer;
  billing: Billing;
}

type FilterStatus = 'all' | 'completed' | 'pending';

export function Transactions() {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<FilterStatus>('all');

  useEffect(() => {
    const fetchTransactions = async () => {
      try {
        const response = await fetch('http://localhost:8000/api/transactions');
        if (!response.ok) {
          throw new Error('Failed to fetch transactions');
        }
        const data = await response.json();
        setTransactions(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'An error occurred');
      } finally {
        setLoading(false);
      }
    };

    fetchTransactions();
  }, []);

  const filteredTransactions = transactions.filter(transaction => {
    const matchesSearch = 
      transaction.order.order_number.toLowerCase().includes(searchQuery.toLowerCase()) ||
      transaction.customer.name.toLowerCase().includes(searchQuery.toLowerCase());
    
    if (filterStatus === 'all') return matchesSearch;
    return matchesSearch && transaction.order.status === filterStatus;
  });

  if (loading) return <div className="loading">Loading transactions...</div>;
  if (error) return <div className="error">Error: {error}</div>;

  return (
    <div className="transactions-container">
      <div className="header">
        <h1>Transactions</h1>
        <button className="close-button">X</button>
      </div>
      
      <div className="controls">
        <div className="search-container">
          <input
            type="text"
            placeholder="Search"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="search-input"
          />
          <SearchIcon className="search-icon" />
        </div>

        <div className="filter-buttons">
          <button
            className={filterStatus === 'all' ? 'active' : ''}
            onClick={() => setFilterStatus('all')}
          >
            All
          </button>
          <button
            className={filterStatus === 'completed' ? 'active' : ''}
            onClick={() => setFilterStatus('completed')}
          >
            Completed
          </button>
          <button
            className={filterStatus === 'pending' ? 'active' : ''}
            onClick={() => setFilterStatus('pending')}
          >
            Pending
          </button>
        </div>
      </div>

      <div className="table-container">
        <table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Order #</th>
              <th>Customer</th>
              <th>Date</th>
              <th>Amount</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {filteredTransactions.map((transaction) => (
              <tr key={transaction.order.id}>
                <td>
                  <div className="id-cell">
                    <span className="user-icon">👤</span>
                    {transaction.order.id}
                  </div>
                </td>
                <td>{transaction.order.order_number}</td>
                <td>{transaction.customer.name}</td>
                <td>{new Date(transaction.order.created_at).toLocaleDateString()}</td>
                <td className="amount">
                  ${transaction.items.reduce((sum, item) => sum + item.price, 0).toFixed(2)}
                </td>
                <td>
                  {transaction.order.status === 'completed' ? (
                    <CheckCircleIcon className="status-icon completed" />
                  ) : (
                    <CancelIcon className="status-icon pending" />
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      
      <div className="footer">
        Records: {filteredTransactions.length}
      </div>
    </div>
  );
} 