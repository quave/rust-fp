// FilterTypes.ts - TypeScript equivalents of the Rust filter model

export enum Operator {
  Equal = '=',
  NotEqual = '!=',
  GreaterThan = '>',
  GreaterThanOrEqual = '>=',
  LessThan = '<',
  LessThanOrEqual = '<=',
  Like = 'like',
  In = 'in',
  NotIn = 'not_in',
  Between = 'between',
  IsNull = 'is_null',
  IsNotNull = 'is_not_null',
}

export type FilterValue = 
  | string
  | number
  | boolean
  | string[]
  | number[]
  | { min: number, max: number };

export interface FilterCondition {
  column: string;
  operator: Operator;
  value: FilterValue;
}

export enum LogicalOperator {
  And = 'and',
  Or = 'or',
}

export interface FilterGroup {
  operator: LogicalOperator;
  conditions: FilterCondition[];
  groups: FilterGroup[];
}

export enum SortDirection {
  Ascending = 'asc',
  Descending = 'desc',
}

export interface SortOrder {
  column: string;
  direction: SortDirection;
}

export interface FilterRequest {
  filter?: FilterGroup;
  sort: SortOrder[];
  limit?: number;
  offset?: number;
}

// Field definition for available filterable fields
export enum FieldType {
  String = 'string',
  Number = 'number',
  DateTime = 'datetime',
  Boolean = 'boolean',
}

export interface Field {
  name: string;
  label: string;
  type: FieldType;
  options?: string[] | number[]; // For fields with predefined options
  sortable?: boolean;
  filterable?: boolean;
}

// Transaction field definitions
export const transactionFields: Field[] = [
  // Fields directly on the root model (orders table)
  { name: 'id', label: 'Order ID', type: FieldType.Number, sortable: true, filterable: true },
  { name: 'transaction_id', label: 'Transaction ID', type: FieldType.Number, sortable: true, filterable: true },
  { name: 'order_number', label: 'Order Number', type: FieldType.String, sortable: true, filterable: true },
  { name: 'delivery_type', label: 'Delivery Type', type: FieldType.String, sortable: true, filterable: true },
  { name: 'delivery_details', label: 'Delivery Details', type: FieldType.String, sortable: true, filterable: true },
  { name: 'created_at', label: 'Created At', type: FieldType.DateTime, sortable: true, filterable: true },
  
  // Fields from relationships using the exact relation names defined in the model
  // These relation names are defined using the #[relation] attribute on DbOrder
  
  // "customer" relation - points to customers table
  { name: 'customer.id', label: 'Customer ID', type: FieldType.Number, sortable: true, filterable: true },
  { name: 'customer.name', label: 'Customer Name', type: FieldType.String, sortable: true, filterable: true },
  { name: 'customer.email', label: 'Customer Email', type: FieldType.String, sortable: true, filterable: true },
  { name: 'customer.created_at', label: 'Customer Created At', type: FieldType.DateTime, sortable: true, filterable: true },
  
  // "billing" relation - points to billing_data table
  { name: 'billing.id', label: 'Billing ID', type: FieldType.Number, sortable: true, filterable: true },
  { name: 'billing.payment_type', label: 'Payment Type', type: FieldType.String, sortable: true, filterable: true },
  { name: 'billing.payment_details', label: 'Payment Details', type: FieldType.String, sortable: true, filterable: true },
  { name: 'billing.billing_address', label: 'Billing Address', type: FieldType.String, sortable: true, filterable: true },
  { name: 'billing.created_at', label: 'Billing Created At', type: FieldType.DateTime, sortable: true, filterable: true },
  
  // "items" relation - points to order_items table (one-to-many)
  { name: 'items.id', label: 'Item ID', type: FieldType.Number, sortable: true, filterable: true },
  { name: 'items.name', label: 'Item Name', type: FieldType.String, sortable: true, filterable: true },
  { name: 'items.category', label: 'Item Category', type: FieldType.String, sortable: true, filterable: true },
  { name: 'items.price', label: 'Item Price', type: FieldType.Number, sortable: true, filterable: true },
  { name: 'items.created_at', label: 'Item Created At', type: FieldType.DateTime, sortable: true, filterable: true },
]; 