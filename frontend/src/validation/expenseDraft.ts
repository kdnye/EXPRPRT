import { ExpenseDraft, ExpenseItemDraft } from '../types/expenseDraft';

export type ExpenseItemFieldKey = 'expense_date' | 'amount_cents' | 'receipts';
export type ReceiptFieldKey = 'file_key' | 'file_name' | 'size_bytes';

export interface ExpenseItemValidationOptions {
  reportingPeriodStart?: string;
  reportingPeriodEnd?: string;
  maxReceipts?: number;
  maxReceiptBytes?: number;
}

export interface ExpenseItemErrorDetail {
  fields: Partial<Record<ExpenseItemFieldKey, string[]>>;
  receipts: Record<string, Partial<Record<ReceiptFieldKey, string[]>>>;
}

export interface DraftValidationResult {
  formErrors: Record<string, string[]>;
  itemErrors: Record<string, ExpenseItemErrorDetail>;
}

const isWithinPeriod = (date: string, start?: string, end?: string) => {
  if (!date) {
    return false;
  }
  if (start && date < start) {
    return false;
  }
  if (end && date > end) {
    return false;
  }
  return true;
};

const hasReceiptErrors = (detail: ExpenseItemErrorDetail) =>
  Object.values(detail.receipts).some((fields) =>
    Object.values(fields).some((messages) => messages && messages.length > 0)
  );

export const validateExpenseItem = (
  item: ExpenseItemDraft,
  options: ExpenseItemValidationOptions
): ExpenseItemErrorDetail => {
  const detail: ExpenseItemErrorDetail = { fields: {}, receipts: {} };

  if (!item.expenseDate) {
    detail.fields.expense_date = ['Date is required'];
  } else if (!isWithinPeriod(item.expenseDate, options.reportingPeriodStart, options.reportingPeriodEnd)) {
    detail.fields.expense_date = ['Date must fall within the reporting period'];
  }

  const amountValue = item.amount.trim();
  const parsed = Number.parseFloat(amountValue);
  if (!amountValue) {
    detail.fields.amount_cents = ['Amount is required'];
  } else if (Number.isNaN(parsed)) {
    detail.fields.amount_cents = ['Amount must be a valid number'];
  } else if (Math.round(parsed * 100) <= 0) {
    detail.fields.amount_cents = ['Amount must be greater than 0'];
  }

  if (options.maxReceipts && item.receipts.length > options.maxReceipts) {
    detail.fields.receipts = [`Only ${options.maxReceipts} receipt(s) allowed per item`];
  }

  item.receipts.forEach((receipt) => {
    const receiptErrors: Partial<Record<ReceiptFieldKey, string[]>> = {};
    if (!receipt.fileKey.trim()) {
      receiptErrors.file_key = ['File key is required'];
    }
    if (!receipt.fileName.trim()) {
      receiptErrors.file_name = ['File name is required'];
    }
    if (!receipt.mimeType.trim()) {
      receiptErrors.size_bytes = [
        ...(receiptErrors.size_bytes ?? []),
        'MIME type is required'
      ];
    }
    if (receipt.size <= 0) {
      receiptErrors.size_bytes = [...(receiptErrors.size_bytes ?? []), 'File size must be greater than 0'];
    } else if (options.maxReceiptBytes && receipt.size > options.maxReceiptBytes) {
      receiptErrors.size_bytes = [
        ...(receiptErrors.size_bytes ?? []),
        `File size exceeds ${(options.maxReceiptBytes / (1024 * 1024)).toFixed(1)} MB limit`
      ];
    }

    if (Object.keys(receiptErrors).length > 0) {
      detail.receipts[receipt.id] = receiptErrors;
    }
  });

  return detail;
};

export const validateDraft = (
  draft: ExpenseDraft,
  options: ExpenseItemValidationOptions
): DraftValidationResult => {
  const formErrors: Record<string, string[]> = {};
  const itemErrors: Record<string, ExpenseItemErrorDetail> = {};

  if (!draft.reportingPeriodStart) {
    formErrors.reporting_period_start = ['Reporting period start is required'];
  }
  if (!draft.reportingPeriodEnd) {
    formErrors.reporting_period_end = ['Reporting period end is required'];
  }
  if (
    draft.reportingPeriodStart &&
    draft.reportingPeriodEnd &&
    draft.reportingPeriodStart > draft.reportingPeriodEnd
  ) {
    formErrors.reporting_period_end = ['End date must be on or after the start date'];
  }
  if (!draft.currency.trim()) {
    formErrors.currency = ['Currency is required'];
  }
  if (draft.items.length === 0) {
    formErrors.items = ['Add at least one expense item'];
  }

  draft.items.forEach((item) => {
    const detail = validateExpenseItem(item, options);
    const hasFieldErrors = Object.values(detail.fields).some(
      (messages) => messages && messages.length > 0
    );
    if (hasFieldErrors || hasReceiptErrors(detail)) {
      itemErrors[item.id] = detail;
    }
  });

  return { formErrors, itemErrors };
};

export const hasValidationIssues = (result: DraftValidationResult) =>
  Object.keys(result.formErrors).length > 0 || Object.keys(result.itemErrors).length > 0;
