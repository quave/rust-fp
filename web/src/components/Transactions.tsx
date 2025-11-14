import { useEffect, useState } from 'react';
import './Transactions.css';
import SearchIcon from '@mui/icons-material/Search';
import CheckCircleIcon from '@mui/icons-material/CheckCircle';
import CancelIcon from '@mui/icons-material/Cancel';
import { FilterBuilder } from './FilterBuilder';
import { FilterRequest, Field } from './FilterTypes';
import { buildFilters, graphqlFetch, introspectFilters } from '../lib/graphql';

interface UITransaction {
  id: number;
  orderNumber: string;
  customerName: string;
  createdAt: string;
  totalAmount: number;
  status: 'completed' | 'pending';
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
  const [transactions, setTransactions] = useState<UITransaction[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<FilterStatus>('all');
  const [showAdvancedFilters, setShowAdvancedFilters] = useState(false);
  const [filterRequest, setFilterRequest] = useState<FilterRequest>({ sort: [] });
  const [fields, setFields] = useState<Field[]>([]);
  
  // Batch labeling state
  const [batchLabelMode, setBatchLabelMode] = useState(false);
  const [selectedTransactions, setSelectedTransactions] = useState<number[]>([]);
  const [selectedFraudLevel, setSelectedFraudLevel] = useState<FraudLevel>(FraudLevel.NoFraud);
  const [selectedFraudCategory, setSelectedFraudCategory] = useState<string>(fraudCategories[4]);
  const [labelingInProgress, setLabelingInProgress] = useState(false);

  useEffect(() => {
    const bootstrap = async () => {
      try {
        setLoading(true);
        const dynFields = await introspectFilters();
        setFields(dynFields);
        await fetchTransactions();
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to initialize');
      } finally {
        setLoading(false);
      }
    };
    bootstrap();
  }, []);

  const fetchTransactions = async (customFilter?: FilterRequest) => {
    setLoading(true);
    try {
      // Always pass an object for GraphQL `filters` arg to satisfy `.object()` on backend
      const filters = customFilter ? (buildFilters(customFilter, fields) || {}) : {};
      const QUERY = `
        query Transactions($filters: InputFilters) {
          transaction(filters: $filters) {
            id
            processing_complete
            created_at
            payload {
              order_number
              created_at
              total_amount
              customer_name
            }
          }
        }
      `;
      type GqlTx = {
        transaction: Array<{
          id: number;
          processing_complete: boolean;
          created_at: string;
          payload: {
            order_number: string;
            created_at: string;
            total_amount: number;
            customer_name: string;
          };
        }>;
      };
      const data = await graphqlFetch<GqlTx>(QUERY, { filters });
      const mapped: UITransaction[] = data.transaction.map(t => ({
        id: t.id,
        orderNumber: t.payload.order_number,
        customerName: t.payload.customer_name,
        createdAt: t.payload.created_at ?? t.created_at,
        totalAmount: Number(t.payload.total_amount ?? 0),
        status: t.processing_complete ? 'completed' : 'pending',
      }));
      setTransactions(mapped);
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
      setSelectedTransactions(filteredTransactions.map(t => t.id));
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
      await fetchTransactions(filterRequest);
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
      transaction.orderNumber.toLowerCase().includes(searchQuery.toLowerCase()) ||
      transaction.customerName.toLowerCase().includes(searchQuery.toLowerCase());
    
    if (filterStatus === 'all') return matchesSearch;
    return matchesSearch && transaction.status === filterStatus;
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
          fields={fields}
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
              <tr key={transaction.id}>
                {batchLabelMode && (
                  <td>
                    <input 
                      type="checkbox" 
                      checked={selectedTransactions.includes(transaction.id)}
                      onChange={() => toggleTransactionSelection(transaction.id)}
                    />
                  </td>
                )}
                <td>
                  <div className="id-cell">
                    <span className="user-icon">👤</span>
                    {transaction.id}
                  </div>
                </td>
                <td>{transaction.orderNumber}</td>
                <td>{transaction.customerName}</td>
                <td>{new Date(transaction.createdAt).toLocaleDateString()}</td>
                <td className="amount">
                  ${transaction.totalAmount.toFixed(2)}
                </td>
                <td>
                  {transaction.status === 'completed' ? (
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