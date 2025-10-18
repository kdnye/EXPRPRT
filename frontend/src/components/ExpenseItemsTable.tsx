import { ChangeEvent, Fragment, useCallback, useEffect, useMemo } from 'react';
import { ExpenseItemDraft, ReceiptDraft } from '../types/expenseDraft';
import { ExpenseCategory, EXPENSE_CATEGORY_OPTIONS } from '../types/expenses';
import {
  ExpenseItemErrorDetail,
  ExpenseItemValidationOptions,
  validateExpenseItem
} from '../validation/expenseDraft';
import './ExpenseItemsTable.css';

const DEFAULT_CATEGORY: ExpenseCategory = 'meal';

const createEmptyItem = (): ExpenseItemDraft => ({
  id: crypto.randomUUID(),
  expenseDate: '',
  category: DEFAULT_CATEGORY,
  description: '',
  attendees: '',
  location: '',
  amount: '',
  reimbursable: true,
  paymentMethod: '',
  receipts: []
});

const createReceiptDraft = (file: File): ReceiptDraft => ({
  id: crypto.randomUUID(),
  fileKey: `draft-${crypto.randomUUID()}`,
  fileName: file.name,
  mimeType: file.type || 'application/octet-stream',
  size: file.size
});

const extractFieldErrors = (
  errors: Record<string, ExpenseItemErrorDetail>,
  itemId: string,
  field: keyof ExpenseItemErrorDetail['fields']
) => errors[itemId]?.fields[field] ?? [];

interface ExpenseItemsTableProps {
  items: ExpenseItemDraft[];
  onChange: (items: ExpenseItemDraft[]) => void;
  reportingPeriodStart?: string;
  reportingPeriodEnd?: string;
  maxReceipts?: number;
  maxReceiptBytes?: number;
  backendErrors?: Record<string, string[]>;
  onValidationChange?: (errors: Record<string, ExpenseItemErrorDetail>) => void;
}

const ExpenseItemsTable = ({
  items,
  onChange,
  reportingPeriodStart,
  reportingPeriodEnd,
  maxReceipts,
  maxReceiptBytes,
  backendErrors,
  onValidationChange
}: ExpenseItemsTableProps) => {
  const validationOptions: ExpenseItemValidationOptions = useMemo(
    () => ({
      reportingPeriodStart,
      reportingPeriodEnd,
      maxReceipts,
      maxReceiptBytes
    }),
    [maxReceipts, maxReceiptBytes, reportingPeriodEnd, reportingPeriodStart]
  );

  const computedErrors = useMemo(() => {
    const next: Record<string, ExpenseItemErrorDetail> = {};
    items.forEach((item) => {
      const detail = validateExpenseItem(item, validationOptions);
      const hasFieldErrors = Object.values(detail.fields).some(
        (messages) => messages && messages.length > 0
      );
      const hasReceiptErrors = Object.keys(detail.receipts).length > 0;
      if (hasFieldErrors || hasReceiptErrors) {
        next[item.id] = detail;
      }
    });
    return next;
  }, [items, validationOptions]);

  useEffect(() => {
    onValidationChange?.(computedErrors);
  }, [computedErrors, onValidationChange]);

  const updateItem = useCallback(
    (index: number, patch: Partial<ExpenseItemDraft>) => {
      onChange(
        items.map((item, idx) => (idx === index ? { ...item, ...patch } : item))
      );
    },
    [items, onChange]
  );

  const addItem = useCallback(() => {
    onChange([...items, createEmptyItem()]);
  }, [items, onChange]);

  const removeItem = useCallback(
    (index: number) => {
      onChange(items.filter((_, idx) => idx !== index));
    },
    [items, onChange]
  );

  const handleReceiptUpload = useCallback(
    (index: number, event: ChangeEvent<HTMLInputElement>) => {
      const files = event.target.files;
      if (!files || files.length === 0) {
        return;
      }

      const uploads = Array.from(files).map(createReceiptDraft);
      event.target.value = '';
      updateItem(index, {
        receipts: [...items[index].receipts, ...uploads]
      });
    },
    [items, updateItem]
  );

  const removeReceipt = useCallback(
    (itemIndex: number, receiptId: string) => {
      const item = items[itemIndex];
      updateItem(itemIndex, {
        receipts: item.receipts.filter((receipt) => receipt.id !== receiptId)
      });
    },
    [items, updateItem]
  );

  const backendMessagesForField = useCallback(
    (itemIndex: number, field: string) => {
      const key = `items.${itemIndex}.${field}`;
      return backendErrors?.[key] ?? [];
    },
    [backendErrors]
  );

  const backendMessagesForReceipt = useCallback(
    (itemIndex: number, receiptIndex: number, field: string) => {
      const key = `items.${itemIndex}.receipts.${receiptIndex}.${field}`;
      return backendErrors?.[key] ?? [];
    },
    [backendErrors]
  );

  return (
    <div className="expense-items-table">
      <div className="expense-items-table__header">
        <h3>Expense items</h3>
        <button type="button" className="expense-items-table__add" onClick={addItem}>
          Add expense
        </button>
      </div>
      {items.length === 0 ? (
        <p className="expense-items-table__empty">Add line items to start your report.</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Date</th>
              <th>Category</th>
              <th>Description</th>
              <th>Amount</th>
              <th>Reimbursable</th>
              <th>Payment method</th>
              <th>Receipts</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {items.map((item, index) => {
              const fieldErrors = computedErrors[item.id]?.fields ?? {};
              return (
                <tr key={item.id}>
                  <td>
                    <input
                      type="date"
                      aria-label="Expense date"
                      value={item.expenseDate}
                      onChange={(event) => updateItem(index, { expenseDate: event.target.value })}
                    />
                    {[...extractFieldErrors(computedErrors, item.id, 'expense_date'), ...backendMessagesForField(index, 'expense_date')].map((message) => (
                      <p key={message} className="expense-items-table__error">
                        {message}
                      </p>
                    ))}
                  </td>
                  <td>
                    <select
                      aria-label="Expense category"
                      value={item.category}
                      onChange={(event) =>
                        updateItem(index, {
                          category: event.target.value as ExpenseCategory
                        })
                      }
                    >
                      {EXPENSE_CATEGORY_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td className="expense-items-table__description">
                    <textarea
                      aria-label="Expense description"
                      value={item.description}
                      onChange={(event) => updateItem(index, { description: event.target.value })}
                      placeholder="What was this expense for?"
                    />
                    <input
                      type="text"
                      aria-label="Expense location"
                      value={item.location}
                      onChange={(event) => updateItem(index, { location: event.target.value })}
                      placeholder="Location"
                    />
                    <input
                      type="text"
                      aria-label="Expense attendees"
                      value={item.attendees}
                      onChange={(event) => updateItem(index, { attendees: event.target.value })}
                      placeholder="Attendees"
                    />
                  </td>
                  <td>
                    <input
                      type="number"
                      step="0.01"
                      min="0"
                      aria-label="Expense amount"
                      value={item.amount}
                      onChange={(event) => updateItem(index, { amount: event.target.value })}
                    />
                    {[...extractFieldErrors(computedErrors, item.id, 'amount_cents'), ...backendMessagesForField(index, 'amount_cents')].map((message) => (
                      <p key={message} className="expense-items-table__error">
                        {message}
                      </p>
                    ))}
                  </td>
                  <td className="expense-items-table__checkbox">
                    <label>
                      <input
                        type="checkbox"
                        aria-label="Reimbursable"
                        checked={item.reimbursable}
                        onChange={(event) => updateItem(index, { reimbursable: event.target.checked })}
                      />
                      <span>Yes</span>
                    </label>
                  </td>
                  <td>
                    <input
                      type="text"
                      aria-label="Payment method"
                      value={item.paymentMethod}
                      onChange={(event) => updateItem(index, { paymentMethod: event.target.value })}
                      placeholder="Corporate card, personal, etc."
                    />
                  </td>
                  <td className="expense-items-table__receipts">
                    <div className="expense-items-table__receipt-list">
                      {item.receipts.map((receipt, receiptIndex) => (
                        <Fragment key={receipt.id}>
                          <div className="expense-items-table__receipt">
                            <div>
                              <strong>{receipt.fileName}</strong>
                              <span>{(receipt.size / 1024).toFixed(1)} KB</span>
                            </div>
                            <button
                              type="button"
                              onClick={() => removeReceipt(index, receipt.id)}
                              className="expense-items-table__remove-receipt"
                            >
                              Remove
                            </button>
                          </div>
                          {backendMessagesForReceipt(index, receiptIndex, 'file_key').map((message) => (
                            <p key={`${receipt.id}-file_key-${message}`} className="expense-items-table__error">
                              {message}
                            </p>
                          ))}
                          {backendMessagesForReceipt(index, receiptIndex, 'file_name').map((message) => (
                            <p key={`${receipt.id}-file_name-${message}`} className="expense-items-table__error">
                              {message}
                            </p>
                          ))}
                          {backendMessagesForReceipt(index, receiptIndex, 'size_bytes').map((message) => (
                            <p key={`${receipt.id}-size-${message}`} className="expense-items-table__error">
                              {message}
                            </p>
                          ))}
                        </Fragment>
                      ))}
                    </div>
                    <input
                      type="file"
                      multiple
                      aria-label="Upload receipt"
                      onChange={(event) => handleReceiptUpload(index, event)}
                    />
                    {[...(fieldErrors.receipts ?? []), ...backendMessagesForField(index, 'receipts')].map((message) => (
                      <p key={message} className="expense-items-table__error">
                        {message}
                      </p>
                    ))}
                  </td>
                  <td>
                    <button
                      type="button"
                      onClick={() => removeItem(index)}
                      className="expense-items-table__remove"
                      aria-label="Remove expense"
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
};

export default ExpenseItemsTable;
