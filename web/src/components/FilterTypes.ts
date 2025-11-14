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