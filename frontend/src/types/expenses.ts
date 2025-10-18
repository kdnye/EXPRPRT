export type ExpenseCategory =
  | 'airfare'
  | 'lodging'
  | 'meal'
  | 'ground_transport'
  | 'mileage'
  | 'supplies'
  | 'other';

export const EXPENSE_CATEGORY_OPTIONS: Array<{ value: ExpenseCategory; label: string }> = [
  { value: 'airfare', label: 'Airfare' },
  { value: 'lodging', label: 'Lodging' },
  { value: 'meal', label: 'Meal' },
  { value: 'ground_transport', label: 'Ground Transport' },
  { value: 'mileage', label: 'Mileage' },
  { value: 'supplies', label: 'Supplies' },
  { value: 'other', label: 'Other' }
];
