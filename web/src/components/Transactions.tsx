import { useEffect, useState } from 'react';
import './Transactions.css';
import SearchIcon from '@mui/icons-material/Search';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import CancelIcon from '@mui/icons-material/Cancel';
import { FilterBuilder } from './FilterBuilder';
import { FilterRequest, transactionFields } from './FilterTypes';

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

// Define fraud level enum based on backend
enum FraudLevel {
  Fraud = 'Fraud',
  NoFraud = 'NoFraud',
  BlockedAutomatically = 'BlockedAutomatically',
  AccountTakeover = 'AccountTakeover',
  NotCreditWorthy = 'NotCreditWorthy'
}

// Common fraud categories
const fraudCategories = [
  'Payment Fraud',
  'Identity Theft',
  'Account Takeover',
  'Chargeback',
  'Legitimate Transaction',
  'Other'
];

type FilterStatus = 'all' | 'completed' | 'pending';

export function Transactions() {
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<FilterStatus>('all');
  const [showAdvancedFilters, setShowAdvancedFilters] = useState(false);
  const [filterRequest, setFilterRequest] = useState<FilterRequest>({ sort: [] });
  
  // Batch labeling state
  const [batchLabelMode, setBatchLabelMode] = useState(false);
  const [selectedTransactions, setSelectedTransactions] = useState<number[]>([]);
  const [selectedFraudLevel, setSelectedFraudLevel] = useState<FraudLevel>(FraudLevel.NoFraud);
  const [selectedFraudCategory, setSelectedFraudCategory] = useState<string>(fraudCategories[4]);
  const [labelingInProgress, setLabelingInProgress] = useState(false);

  useEffect(() => {
    fetchTransactions();
  }, []);

  const fetchTransactions = async (customFilter?: FilterRequest) => {
    setLoading(true);
    try {
      // If we have a customFilter, use the new filter endpoint
      const endpoint = customFilter ? 'http://localhost:8000/api/transactions/filter' : 'http://localhost:8000/api/transactions';
      const options = customFilter ? {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(customFilter),
      } : undefined;
      
      const response = await fetch(endpoint, options);
      
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

  const handleApplyFilter = (request: FilterRequest) => {
    setFilterRequest(request);
    fetchTransactions(request);
  };

  const toggleTransactionSelection = (transactionId: number) => {
    setSelectedTransactions(prev => {
      if (prev.includes(transactionId)) {
        return prev.filter(id => id !== transactionId);
      } else {
        return [...prev, transactionId];
      }
    });
  };

  const toggleSelectAll = () => {
    if (selectedTransactions.length === filteredTransactions.length) {
      setSelectedTransactions([]);
    } else {
      setSelectedTransactions(filteredTransactions.map(t => t.order.id));
    }
  };

  const cancelBatchLabel = () => {
    setBatchLabelMode(false);
    setSelectedTransactions([]);
  };

  const saveBatchLabels = async () => {
    if (selectedTransactions.length === 0) return;
    
    setLabelingInProgress(true);
    
    try {
      const response = await fetch('http://localhost:8000/api/transactions/label', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          transaction_ids: selectedTransactions,
          fraud_level: selectedFraudLevel,
          fraud_category: selectedFraudCategory,
          labeled_by: 'Web UI' // Could be replaced with actual user info in the future
        }),
      });
      
      if (!response.ok) {
        throw new Error('Failed to label transactions');
      }
      
      // Success notification could be added here
      console.log('Successfully labeled transactions');
      
      // Exit batch label mode and reset selections
      setBatchLabelMode(false);
      setSelectedTransactions([]);
      
      // Refresh transaction list to show updated labels
      // This is a simple approach - in a real app you might want to
      // just update the local state instead of refetching
      const refreshResponse = await fetch('http://localhost:8000/api/transactions');
      if (refreshResponse.ok) {
        const data = await refreshResponse.json();
        setTransactions(data);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'An error occurred while labeling');
    } finally {
      setLabelingInProgress(false);
    }
  };

  // Use the simpler filter for quick filters
  const filteredTransactions = transactions.filter(transaction => {
    // Skip filtering if we're using advanced filters
    if (filterRequest.filter && filterRequest.filter.conditions && filterRequest.filter.conditions.length > 0 || filterRequest.sort?.length > 0) {
      return true;
    }
    
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
          
          {!batchLabelMode && (
            <>
              <button 
                className="label-button"
                onClick={() => setBatchLabelMode(true)}
              >
                Label
              </button>
              <button 
                className="filter-button"
                onClick={() => setShowAdvancedFilters(!showAdvancedFilters)}
              >
                {showAdvancedFilters ? 'Hide Filters' : 'Advanced Filters'}
              </button>
            </>
          )}
        </div>
      </div>
      
      {showAdvancedFilters && (
        <FilterBuilder 
          fields={transactionFields}
          onApplyFilter={handleApplyFilter}
          initialRequest={filterRequest}
        />
      )}
      
      {batchLabelMode && (
        <div className="batch-label-controls">
          <div className="dropdown-container">
            <label>Fraud Level:</label>
            <select 
              value={selectedFraudLevel} 
              onChange={(e) => setSelectedFraudLevel(e.target.value as FraudLevel)}
            >
              {Object.values(FraudLevel).map(level => (
                <option key={level} value={level}>{level}</option>
              ))}
            </select>
          </div>
          
          <div className="dropdown-container">
            <label>Fraud Category:</label>
            <select 
              value={selectedFraudCategory} 
              onChange={(e) => setSelectedFraudCategory(e.target.value)}
            >
              {fraudCategories.map(category => (
                <option key={category} value={category}>{category}</option>
              ))}
            </select>
          </div>
          
          <div className="batch-label-buttons">
            <button 
              className="save-button" 
              onClick={saveBatchLabels}
              disabled={selectedTransactions.length === 0 || labelingInProgress}
            >
              {labelingInProgress ? 'Saving...' : 'Save'}
            </button>
            <button 
              className="cancel-button" 
              onClick={cancelBatchLabel}
              disabled={labelingInProgress}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      <div className="table-container">
        <table>
          <thead>
            <tr>
              {batchLabelMode && (
                <th>
                  <input 
                    type="checkbox" 
                    checked={selectedTransactions.length === filteredTransactions.length && filteredTransactions.length > 0}
                    onChange={toggleSelectAll}
                  />
                </th>
              )}
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
                {batchLabelMode && (
                  <td>
                    <input 
                      type="checkbox" 
                      checked={selectedTransactions.includes(transaction.order.id)}
                      onChange={() => toggleTransactionSelection(transaction.order.id)}
                    />
                  </td>
                )}
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
        {batchLabelMode && selectedTransactions.length > 0 && (
          <span className="selected-count">, Selected: {selectedTransactions.length}</span>
        )}
      </div>
    </div>
  );
} 