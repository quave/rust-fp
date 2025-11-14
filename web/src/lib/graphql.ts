import { Field, FieldType, FilterRequest, FilterValue, Operator } from '../components/FilterTypes';

export const GRAPHQL_ENDPOINT = 'http://localhost:8000/api/transactions/graphql';

type GraphQLResponse<T> = {
  data?: T;
  errors?: Array<{ message: string }>;
};

export async function graphqlFetch<T>(query: string, variables?: Record<string, unknown>): Promise<T> {
  const res = await fetch(GRAPHQL_ENDPOINT, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ query, variables })
  });
  if (!res.ok) {
    throw new Error(`GraphQL HTTP error: ${res.status}`);
  }
  const payload = (await res.json()) as GraphQLResponse<T>;
  if (payload.errors && payload.errors.length > 0) {
    throw new Error(payload.errors.map(e => e.message).join('; '));
  }
  if (!payload.data) {
    throw new Error('GraphQL: missing data');
  }
  return payload.data;
}

// Introspect InputFilters to build dynamic fields list
export async function introspectFilters(): Promise<Field[]> {
  type Introspection = {
    input: {
      inputFields: Array<{
        name: string;
        type: { name: string | null; ofType: { name: string | null } | null };
      }>;
    } | null;
  };
  const INTROSPECTION_QUERY = `
    query FilterInputs {
      input: __type(name: "InputFilters") {
        inputFields { name type { name ofType { name } } }
      }
    }
  `;
  const data = await graphqlFetch<Introspection>(INTROSPECTION_QUERY);
  const input = data.input;
  if (!input || !input.inputFields) {
    // Fallback to empty list; caller may decide to provide static fields
    return [];
  }

  const mapOpsInputToFieldType = (opsTypeName: string | null): FieldType => {
    switch (opsTypeName) {
      case 'FilterOperatorsStringInput':
        return FieldType.String;
      case 'FilterOperatorsIntInput':
      case 'FilterOperatorsFloatInput':
        return FieldType.Number;
      case 'FilterOperatorsBooleanInput':
        return FieldType.Boolean;
      default:
        return FieldType.String;
    }
  };

  const fields: Field[] = input.inputFields.map(f => {
    const typeName = f.type.name ?? f.type.ofType?.name ?? null;
    const fieldType = mapOpsInputToFieldType(typeName);
    return {
      name: f.name,
      label: prettifyLabel(f.name),
      type: fieldType,
      sortable: true,
      filterable: true
    };
  });
  return fields;
}

function prettifyLabel(name: string): string {
  return name
    .replace(/_/g, ' ')
    .replace(/\b\w/g, c => c.toUpperCase());
}

export function mapOperator(op: Operator): string {
  switch (op) {
    case Operator.Equal:
      return 'eq';
    case Operator.NotEqual:
      return 'not_eq';
    case Operator.GreaterThan:
      return 'gt';
    case Operator.GreaterThanOrEqual:
      return 'gte';
    case Operator.LessThan:
      return 'lt';
    case Operator.LessThanOrEqual:
      return 'lte';
    case Operator.Like:
      return 'contains';
    case Operator.In:
      return 'in';
    case Operator.NotIn:
      return 'not_in';
    case Operator.Between:
      return 'between';
    case Operator.IsNull:
      return 'is_null';
    case Operator.IsNotNull:
      return 'is_not_null';
    default:
      return 'eq';
  }
}

export function mapValueForOp(opKey: string, value: FilterValue, fieldType: FieldType): unknown {
  if (opKey === 'is_null' || opKey === 'is_not_null') {
    return true;
  }
  if (opKey === 'between') {
    // Expect {min, max}
    if (typeof value === 'object' && value !== null && 'min' in value && 'max' in value) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const vAny = value as any;
      return [vAny.min, vAny.max];
    }
    return value;
  }
  if (opKey === 'in' || opKey === 'not_in') {
    if (Array.isArray(value)) return value;
    return [value];
  }
  if (opKey === 'contains' && typeof value === 'string') {
    // Backend expects plain string for contains
    return value;
  }
  // Cast based on field type
  switch (fieldType) {
    case FieldType.Number:
      if (Array.isArray(value)) return value.map(v => Number(v as number));
      return Number(value as number);
    case FieldType.Boolean:
      return Boolean(value as boolean);
    case FieldType.String:
    case FieldType.DateTime:
    default:
      return value;
  }
}

export function buildFilters(request: FilterRequest, fields: Field[]): Record<string, unknown> | undefined {
  const byName: Record<string, Field> = Object.fromEntries(fields.map(f => [f.name, f]));
  const conditions = request.filter?.conditions ?? [];
  if (conditions.length === 0) return undefined;
  const out: Record<string, unknown> = {};
  for (const c of conditions) {
    const fType = byName[c.column]?.type ?? FieldType.String;
    const opKey = mapOperator(c.operator);
    const val = mapValueForOp(opKey, c.value, fType);
    if (opKey === 'in' || opKey === 'not_in') {
      const prev = (out[c.column] as Record<string, unknown> | undefined)?.[opKey] as unknown[] | undefined;
      const merged = Array.from(new Set([...(prev ?? []), ...(Array.isArray(val) ? val : [val])]));
      out[c.column] = { [opKey]: merged };
    } else {
      out[c.column] = { [opKey]: val };
    }
  }
  return out;
}


