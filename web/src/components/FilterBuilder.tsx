import { useState } from 'react';
import {
  Box,
  Button,
  FormControl,
  IconButton,
  InputLabel,
  MenuItem,
  Paper,
  Select,
  Stack,
  TextField,
  Typography,
  Chip,
  Card,
  CardContent,
  Divider,
} from '@mui/material';
import AddIcon from '@mui/icons-material/Add';
import DeleteIcon from '@mui/icons-material/Delete';
import FilterListIcon from '@mui/icons-material/FilterList';
import SortIcon from '@mui/icons-material/Sort';
import {
  Field,
  FieldType,
  FilterCondition,
  FilterGroup,
  FilterRequest,
  LogicalOperator,
  Operator,
  SortDirection,
  SortOrder
} from './FilterTypes';

interface FilterBuilderProps {
  fields: Field[];
  onApplyFilter: (request: FilterRequest) => void;
  initialRequest?: FilterRequest;
}

export function FilterBuilder({ fields, onApplyFilter, initialRequest }: FilterBuilderProps) {
  const [expanded, setExpanded] = useState(false);
  const [filterRequest, setFilterRequest] = useState<FilterRequest>(
    initialRequest || {
      filter: {
        operator: LogicalOperator.And,
        conditions: [],
        groups: []
      },
      sort: [],
      limit: 100,
      offset: 0
    }
  );

  const handleAddCondition = () => {
    // Find first filterable field
    const firstField = fields.find(f => f.filterable !== false) || fields[0];
    const fieldType = firstField?.type || FieldType.String;
    const defaultOperator = getDefaultOperatorForType(fieldType);
    const defaultValue = getDefaultValueForType(fieldType);
    
    // If field has options, use first option as default value
    const initialValue = firstField?.options && firstField.options.length > 0
      ? firstField.options[0]
      : defaultValue;
      
    if (!filterRequest.filter) {
      setFilterRequest({
        ...filterRequest,
        filter: {
          operator: LogicalOperator.And,
          conditions: [{
            column: firstField.name,
            operator: defaultOperator,
            value: initialValue
          }],
          groups: []
        }
      });
    } else {
      setFilterRequest({
        ...filterRequest,
        filter: {
          ...filterRequest.filter,
          conditions: [
            ...filterRequest.filter.conditions,
            {
              column: firstField.name,
              operator: defaultOperator,
              value: initialValue
            }
          ]
        }
      });
    }
  };

  const handleAddSort = () => {
    const sortableFields = fields.filter(f => f.sortable !== false);
    if (sortableFields.length > 0) {
      setFilterRequest({
        ...filterRequest,
        sort: [...filterRequest.sort, { column: sortableFields[0].name, direction: SortDirection.Ascending }]
      });
    }
  };

  const handleRemoveCondition = (index: number) => {
    if (filterRequest.filter) {
      const newConditions = [...filterRequest.filter.conditions];
      newConditions.splice(index, 1);
      setFilterRequest({
        ...filterRequest,
        filter: {
          ...filterRequest.filter,
          conditions: newConditions
        }
      });
    }
  };

  const handleRemoveSort = (index: number) => {
    const newSort = [...filterRequest.sort];
    newSort.splice(index, 1);
    setFilterRequest({
      ...filterRequest,
      sort: newSort
    });
  };

  const handleConditionChange = (index: number, field: keyof FilterCondition, value: any) => {
    if (filterRequest.filter) {
      const newConditions = [...filterRequest.filter.conditions];

      // Special handling when changing the column - reset operator and value based on field type
      if (field === 'column') {
        const fieldDef = fields.find(f => f.name === value);
        const fieldType = fieldDef?.type || FieldType.String;
        const defaultOperator = getDefaultOperatorForType(fieldType);
        const defaultValue = getDefaultValueForType(fieldType);
        
        // If the field has predefined options, select the first option as default
        const finalValue = fieldDef?.options && fieldDef.options.length > 0
          ? fieldDef.options[0]
          : defaultValue;
        
        newConditions[index] = {
          column: value as string,
          operator: defaultOperator,
          value: finalValue
        };
      } else {
        newConditions[index] = {
          ...newConditions[index],
          [field]: value
        };
      }

      setFilterRequest({
        ...filterRequest,
        filter: {
          ...filterRequest.filter,
          conditions: newConditions
        }
      });
    }
  };

  const handleSortChange = (index: number, field: keyof SortOrder, value: any) => {
    const newSort = [...filterRequest.sort];
    newSort[index] = {
      ...newSort[index],
      [field]: value
    };
    setFilterRequest({
      ...filterRequest,
      sort: newSort
    });
  };

  const handleOperatorChange = (newOperator: LogicalOperator) => {
    if (filterRequest.filter) {
      setFilterRequest({
        ...filterRequest,
        filter: {
          ...filterRequest.filter,
          operator: newOperator
        }
      });
    }
  };

  const handleApplyFilter = () => {
    onApplyFilter(filterRequest);
  };

  const handleClearFilter = () => {
    setFilterRequest({
      filter: undefined,
      sort: [],
      limit: 100,
      offset: 0
    });
    onApplyFilter({
      filter: undefined,
      sort: [],
      limit: 100,
      offset: 0
    });
  };

  // Helper function to determine the available operators for a field type
  const getOperatorsForType = (type: FieldType): Operator[] => {
    switch (type) {
      case FieldType.String:
        return [
          Operator.Equal,
          Operator.NotEqual,
          Operator.Like,
          Operator.In,
          Operator.NotIn,
          Operator.IsNull,
          Operator.IsNotNull
        ];
      case FieldType.Number:
        return [
          Operator.Equal,
          Operator.NotEqual,
          Operator.GreaterThan,
          Operator.GreaterThanOrEqual,
          Operator.LessThan,
          Operator.LessThanOrEqual,
          Operator.Between,
          Operator.In,
          Operator.NotIn,
          Operator.IsNull,
          Operator.IsNotNull
        ];
      case FieldType.DateTime:
        return [
          Operator.Equal,
          Operator.NotEqual,
          Operator.GreaterThan,
          Operator.GreaterThanOrEqual,
          Operator.LessThan,
          Operator.LessThanOrEqual,
          Operator.Between,
          Operator.IsNull,
          Operator.IsNotNull
        ];
      case FieldType.Boolean:
        return [Operator.Equal, Operator.NotEqual, Operator.IsNull, Operator.IsNotNull];
      default:
        return [Operator.Equal, Operator.NotEqual];
    }
  };

  // Helper function to get default operator for a field type
  const getDefaultOperatorForType = (type: FieldType): Operator => {
    switch (type) {
      case FieldType.String:
        return Operator.Like;
      case FieldType.Number:
      case FieldType.DateTime:
        return Operator.Equal;
      case FieldType.Boolean:
        return Operator.Equal;
      default:
        return Operator.Equal;
    }
  };

  // Helper function to get default value for a field type
  const getDefaultValueForType = (type: FieldType): any => {
    switch (type) {
      case FieldType.String:
        return '';
      case FieldType.Number:
        return 0;
      case FieldType.DateTime:
        // Return current date-time in ISO format for datetime fields
        return new Date().toISOString().slice(0, 16);
      case FieldType.Boolean:
        return true;
      default:
        return '';
    }
  };

  // Render value input based on operator and field type
  const renderValueInput = (condition: FilterCondition, index: number) => {
    const field = fields.find(f => f.name === condition.column);
    
    if (!field) return null;
    
    // For IS NULL and IS NOT NULL operators, no value input needed
    if (condition.operator === Operator.IsNull || condition.operator === Operator.IsNotNull) {
      return null;
    }

    // For operators that work with arrays (IN, NOT IN)
    if (condition.operator === Operator.In || condition.operator === Operator.NotIn) {
      return (
        <TextField
          fullWidth
          label="Values (comma separated)"
          value={Array.isArray(condition.value) ? condition.value.join(',') : condition.value}
          onChange={(e) => {
            const values = e.target.value.split(',').map(v => {
              // Convert to number if field is numeric
              return field.type === FieldType.Number ? Number(v.trim()) : v.trim();
            });
            handleConditionChange(index, 'value', values);
          }}
        />
      );
    }

    // For BETWEEN operator
    if (condition.operator === Operator.Between) {
      const range = typeof condition.value === 'object' && 'min' in condition.value
        ? condition.value
        : { min: field.type === FieldType.Number ? 0 : '', max: field.type === FieldType.Number ? 0 : '' };
        
      return (
        <Stack direction="row" spacing={1}>
          <TextField
            label="Min"
            type={field.type === FieldType.Number ? 'number' : 'text'}
            value={range.min}
            onChange={(e) => {
              const newValue = field.type === FieldType.Number ? 
                (e.target.value === '' ? 0 : Number(e.target.value)) : 
                e.target.value;
              
              handleConditionChange(index, 'value', {
                ...range,
                min: newValue
              });
            }}
          />
          <TextField
            label="Max"
            type={field.type === FieldType.Number ? 'number' : 'text'}
            value={range.max}
            onChange={(e) => {
              const newValue = field.type === FieldType.Number ? 
                (e.target.value === '' ? 0 : Number(e.target.value)) : 
                e.target.value;
                
              handleConditionChange(index, 'value', {
                ...range,
                max: newValue
              });
            }}
          />
        </Stack>
      );
    }

    // For fields with predefined options
    if (field.options && field.options.length > 0) {
      return (
        <FormControl fullWidth>
          <InputLabel>Value</InputLabel>
          <Select
            value={condition.value}
            label="Value"
            onChange={(e) => handleConditionChange(index, 'value', e.target.value)}
          >
            {field.options.map((option) => (
              <MenuItem key={option} value={option}>
                {option}
              </MenuItem>
            ))}
          </Select>
        </FormControl>
      );
    }

    // Default input based on field type
    switch (field.type) {
      case FieldType.Number:
        return (
          <TextField
            fullWidth
            label="Value"
            type="number"
            value={condition.value === 0 ? '0' : condition.value}
            onChange={(e) => {
              // Handle empty input as 0 or convert to number
              const value = e.target.value === '' ? 0 : Number(e.target.value);
              handleConditionChange(index, 'value', value);
            }}
          />
        );
      case FieldType.DateTime:
        return (
          <TextField
            fullWidth
            label="Value"
            type="datetime-local"
            InputLabelProps={{ shrink: true }}
            value={condition.value}
            onChange={(e) => handleConditionChange(index, 'value', e.target.value)}
          />
        );
      case FieldType.Boolean:
        return (
          <FormControl fullWidth>
            <InputLabel>Value</InputLabel>
            <Select
              value={condition.value === true ? 'true' : 'false'}
              label="Value"
              onChange={(e) => handleConditionChange(index, 'value', e.target.value === 'true')}
            >
              <MenuItem value="true">True</MenuItem>
              <MenuItem value="false">False</MenuItem>
            </Select>
          </FormControl>
        );
      case FieldType.String:
      default:
        return (
          <TextField
            fullWidth
            label="Value"
            value={condition.value === null || condition.value === undefined ? '' : condition.value}
            onChange={(e) => handleConditionChange(index, 'value', e.target.value)}
          />
        );
    }
  };

  // Summary of active filters for collapsed view
  const renderFilterSummary = () => {
    if (!filterRequest.filter || filterRequest.filter.conditions.length === 0) {
      return <Typography variant="body2" color="text.secondary">No filters applied</Typography>;
    }

    return (
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
        {filterRequest.filter.conditions.map((condition, index) => {
          const field = fields.find(f => f.name === condition.column);
          let valueText = '';
          
          if (condition.operator === Operator.IsNull) {
            valueText = 'is empty';
          } else if (condition.operator === Operator.IsNotNull) {
            valueText = 'is not empty';
          } else if (typeof condition.value === 'object' && 'min' in condition.value) {
            // Handle range values safely
            const min = condition.value.min !== 0 && !condition.value.min ? 'any' : condition.value.min;
            const max = condition.value.max !== 0 && !condition.value.max ? 'any' : condition.value.max;
            valueText = `${min} - ${max}`;
          } else if (Array.isArray(condition.value)) {
            // Join array values safely
            valueText = condition.value.map(v => v === null ? 'empty' : String(v)).join(', ');
          } else if (condition.value === null || condition.value === undefined) {
            // Handle null/undefined values
            valueText = 'empty';
          } else {
            // Convert any value to string
            valueText = String(condition.value);
          }
          
          return (
            <Chip
              key={index}
              label={`${field?.label || condition.column} ${condition.operator} ${valueText}`}
              onDelete={() => handleRemoveCondition(index)}
              color="primary"
              variant="outlined"
            />
          );
        })}
      </Box>
    );
  };

  // Summary of active sorts for collapsed view
  const renderSortSummary = () => {
    if (filterRequest.sort.length === 0) {
      return <Typography variant="body2" color="text.secondary">No sorting applied</Typography>;
    }

    return (
      <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
        {filterRequest.sort.map((sort, index) => {
          const field = fields.find(f => f.name === sort.column);
          return (
            <Chip
              key={index}
              label={`${field?.label || sort.column} ${sort.direction}`}
              onDelete={() => handleRemoveSort(index)}
              color="secondary"
              variant="outlined"
            />
          );
        })}
      </Box>
    );
  };

  return (
    <Paper sx={{ mb: 2, p: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 2 }}>
        <Typography variant="h6" display="flex" alignItems="center" gap={1}>
          <FilterListIcon /> Advanced Filters
        </Typography>
        <Button onClick={() => setExpanded(!expanded)}>
          {expanded ? 'Collapse' : 'Expand'} Filters
        </Button>
      </Box>

      {!expanded ? (
        <Stack spacing={2}>
          <Box>
            <Typography variant="subtitle2">Filters:</Typography>
            {renderFilterSummary()}
          </Box>
          <Box>
            <Typography variant="subtitle2">Sort:</Typography>
            {renderSortSummary()}
          </Box>
          <Box sx={{ display: 'flex', justifyContent: 'flex-end', gap: 1 }}>
            <Button variant="outlined" onClick={handleClearFilter}>
              Clear
            </Button>
            <Button variant="contained" onClick={handleApplyFilter}>
              Apply
            </Button>
          </Box>
        </Stack>
      ) : (
        <Stack spacing={3}>
          {/* Filter conditions section */}
          <Card variant="outlined">
            <CardContent>
              <Typography variant="subtitle1" sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <FilterListIcon sx={{ mr: 1 }} /> Filter Conditions
              </Typography>
              
              {/* Logical operator selection */}
              {filterRequest.filter && filterRequest.filter.conditions.length > 1 && (
                <FormControl sx={{ mb: 2, minWidth: 120 }} size="small">
                  <InputLabel>Logic</InputLabel>
                  <Select
                    value={filterRequest.filter.operator}
                    label="Logic"
                    onChange={(e) => handleOperatorChange(e.target.value as LogicalOperator)}
                  >
                    <MenuItem value={LogicalOperator.And}>AND</MenuItem>
                    <MenuItem value={LogicalOperator.Or}>OR</MenuItem>
                  </Select>
                </FormControl>
              )}
              
              {/* Render filter conditions */}
              {filterRequest.filter && filterRequest.filter.conditions.map((condition, index) => {
                const field = fields.find(f => f.name === condition.column);
                return (
                  <Box 
                    key={index} 
                    sx={{ 
                      mb: 2, 
                      p: 2, 
                      border: '1px solid', 
                      borderColor: 'divider',
                      borderRadius: 1,
                      position: 'relative'
                    }}
                  >
                    <IconButton 
                      size="small" 
                      onClick={() => handleRemoveCondition(index)}
                      sx={{ position: 'absolute', top: 8, right: 8 }}
                    >
                      <DeleteIcon />
                    </IconButton>
                    
                    <Stack spacing={2}>
                      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
                        {/* Field selection */}
                        <FormControl fullWidth>
                          <InputLabel>Field</InputLabel>
                          <Select
                            value={condition.column}
                            label="Field"
                            onChange={(e) => handleConditionChange(index, 'column', e.target.value)}
                          >
                            {fields.filter(f => f.filterable !== false).map((field) => (
                              <MenuItem key={field.name} value={field.name}>
                                {field.label}
                              </MenuItem>
                            ))}
                          </Select>
                        </FormControl>
                        
                        {/* Operator selection */}
                        <FormControl fullWidth>
                          <InputLabel>Operator</InputLabel>
                          <Select
                            value={condition.operator}
                            label="Operator"
                            onChange={(e) => handleConditionChange(index, 'operator', e.target.value)}
                          >
                            {getOperatorsForType(field?.type || FieldType.String).map((op) => (
                              <MenuItem key={op} value={op}>
                                {op}
                              </MenuItem>
                            ))}
                          </Select>
                        </FormControl>
                      </Stack>
                      
                      {/* Value input */}
                      {renderValueInput(condition, index)}
                    </Stack>
                  </Box>
                );
              })}
              
              <Button 
                startIcon={<AddIcon />} 
                onClick={handleAddCondition}
                variant="outlined"
                fullWidth
              >
                Add Condition
              </Button>
            </CardContent>
          </Card>
          
          {/* Sort section */}
          <Card variant="outlined">
            <CardContent>
              <Typography variant="subtitle1" sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
                <SortIcon sx={{ mr: 1 }} /> Sorting
              </Typography>
              
              {filterRequest.sort.map((sort, index) => (
                <Box 
                  key={index} 
                  sx={{ 
                    mb: 2, 
                    p: 2, 
                    border: '1px solid', 
                    borderColor: 'divider',
                    borderRadius: 1,
                    position: 'relative'
                  }}
                >
                  <IconButton 
                    size="small" 
                    onClick={() => handleRemoveSort(index)}
                    sx={{ position: 'absolute', top: 8, right: 8 }}
                  >
                    <DeleteIcon />
                  </IconButton>
                  
                  <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
                    {/* Sort field selection */}
                    <FormControl fullWidth>
                      <InputLabel>Field</InputLabel>
                      <Select
                        value={sort.column}
                        label="Field"
                        onChange={(e) => handleSortChange(index, 'column', e.target.value)}
                      >
                        {fields.filter(f => f.sortable !== false).map((field) => (
                          <MenuItem key={field.name} value={field.name}>
                            {field.label}
                          </MenuItem>
                        ))}
                      </Select>
                    </FormControl>
                    
                    {/* Sort direction */}
                    <FormControl fullWidth>
                      <InputLabel>Direction</InputLabel>
                      <Select
                        value={sort.direction}
                        label="Direction"
                        onChange={(e) => handleSortChange(index, 'direction', e.target.value)}
                      >
                        <MenuItem value={SortDirection.Ascending}>Ascending</MenuItem>
                        <MenuItem value={SortDirection.Descending}>Descending</MenuItem>
                      </Select>
                    </FormControl>
                  </Stack>
                </Box>
              ))}
              
              <Button 
                startIcon={<AddIcon />} 
                onClick={handleAddSort}
                variant="outlined"
                fullWidth
              >
                Add Sort
              </Button>
            </CardContent>
          </Card>
          
          {/* Pagination section */}
          <Card variant="outlined">
            <CardContent>
              <Typography variant="subtitle1" sx={{ mb: 2 }}>
                Pagination
              </Typography>
              
              <Stack direction={{ xs: 'column', sm: 'row' }} spacing={2}>
                <TextField
                  label="Limit"
                  type="number"
                  value={filterRequest.limit || 100}
                  onChange={(e) => setFilterRequest({
                    ...filterRequest,
                    limit: Number(e.target.value)
                  })}
                  InputProps={{ inputProps: { min: 1 } }}
                />
                <TextField
                  label="Offset"
                  type="number"
                  value={filterRequest.offset || 0}
                  onChange={(e) => setFilterRequest({
                    ...filterRequest,
                    offset: Number(e.target.value)
                  })}
                  InputProps={{ inputProps: { min: 0 } }}
                />
              </Stack>
            </CardContent>
          </Card>
          
          <Divider />
          
          {/* Action buttons */}
          <Box sx={{ display: 'flex', justifyContent: 'flex-end', gap: 1 }}>
            <Button variant="outlined" onClick={handleClearFilter}>
              Clear All
            </Button>
            <Button variant="contained" onClick={handleApplyFilter}>
              Apply Filters
            </Button>
          </Box>
        </Stack>
      )}
    </Paper>
  );
} 