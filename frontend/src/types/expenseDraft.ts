import { ExpenseCategory } from './expenses';

export interface ReceiptDraft {
  id: string;
  fileKey: string;
  fileName: string;
  mimeType: string;
  size: number;
}

export interface ExpenseItemDraft {
  id: string;
  expenseDate: string;
  category: ExpenseCategory;
  description: string;
  attendees: string;
  location: string;
  amount: string;
  reimbursable: boolean;
  paymentMethod: string;
  receipts: ReceiptDraft[];
}

export interface ExpenseDraft {
  reportingPeriodStart: string;
  reportingPeriodEnd: string;
  currency: string;
  items: ExpenseItemDraft[];
}
