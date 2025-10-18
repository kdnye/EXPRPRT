/**
 * Employee self-service view for drafting and submitting new expense reports.
 *
 * - Calls `POST /expenses/reports` to create a draft report and
 *   `POST /expenses/reports/:id/submit` to hand the draft to manager review.
 *   Those endpoints are implemented by `backend/src/api/rest/expenses.rs`
 *   backed by the workflows in `backend/src/services/expenses.rs`.
 * - Surfaces high-level policy messaging for offline drafting and validation
 *   in line with POLICY.md §"Approvals and Reimbursement Process", ensuring
 *   employees understand that policy checks run prior to manager approval.
 * - Pairs with `frontend/src/hooks/useExpenseDraft.ts` for offline-ready draft
 *   persistence so users can work without a network connection before
 *   submission.
 */
import axios from 'axios';
import { useCallback, useMemo, useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { z } from 'zod';
import { request } from '@/api/client';
import ExpenseItemsTable from '../components/ExpenseItemsTable';
import SummaryCard from '../components/SummaryCard';
import useExpenseDraft from '../hooks/useExpenseDraft';
import { ExpenseDraft } from '../types/expenseDraft';
import {
  hasValidationIssues,
  validateDraft
} from '../validation/expenseDraft';
import './EmployeePortal.css';

const createReportResponseSchema = z.object({
  report: z.object({
    id: z.string()
  })
});

const EXPENSE_DRAFT_ID = 'employee-portal:new-expense-report';
const RECEIPT_LIMIT = {
  maxReceipts: 10,
  maxReceiptBytes: 5 * 1024 * 1024
};

const normalizeString = (value: string) => (value.trim().length > 0 ? value : undefined);

const toCents = (amount: string) => {
  const parsed = Number.parseFloat(amount);
  if (Number.isNaN(parsed)) {
    return 0;
  }
  return Math.round(parsed * 100);
};

const EmployeePortal = () => {
  const createInitialDraft = useCallback<() => ExpenseDraft>(
    () => ({
      reportingPeriodStart: '',
      reportingPeriodEnd: '',
      currency: 'USD',
      items: []
    }),
    []
  );

  const [draft, setDraft, resetDraft] = useExpenseDraft<ExpenseDraft>(
    EXPENSE_DRAFT_ID,
    createInitialDraft
  );
  const [backendErrors, setBackendErrors] = useState<Record<string, string[]>>({});
  const [submissionError, setSubmissionError] = useState<string | null>(null);
  const [showClientErrors, setShowClientErrors] = useState(false);

  const validationOptions = useMemo(
    () => ({
      reportingPeriodStart: draft.reportingPeriodStart || undefined,
      reportingPeriodEnd: draft.reportingPeriodEnd || undefined,
      maxReceipts: RECEIPT_LIMIT.maxReceipts,
      maxReceiptBytes: RECEIPT_LIMIT.maxReceiptBytes
    }),
    [draft.reportingPeriodEnd, draft.reportingPeriodStart]
  );

  const draftValidation = useMemo(
    () => validateDraft(draft, validationOptions),
    [draft, validationOptions]
  );
  const hasClientErrors = hasValidationIssues(draftValidation);

  const createReportMutation = useMutation({
    mutationFn: async (form: ExpenseDraft) => {
      const payload = await request<unknown>('post', '/expenses/reports', {
        reporting_period_start: form.reportingPeriodStart,
        reporting_period_end: form.reportingPeriodEnd,
        currency: form.currency,
        items: form.items.map((item) => ({
          expense_date: item.expenseDate,
          category: item.category,
          description: normalizeString(item.description),
          attendees: normalizeString(item.attendees),
          location: normalizeString(item.location),
          amount_cents: toCents(item.amount),
          reimbursable: item.reimbursable,
          payment_method: normalizeString(item.paymentMethod),
          receipts: item.receipts.map((receipt) => ({
            file_key: receipt.fileKey,
            file_name: receipt.fileName,
            mime_type: receipt.mimeType,
            size_bytes: receipt.size
          }))
        }))
      });

      return createReportResponseSchema.parse(payload).report.id;
    }
  });

  const submitReportMutation = useMutation({
    mutationFn: (id: string) => request<unknown>('post', `/expenses/reports/${id}/submit`)
  });

  const handleSubmit = useCallback(async () => {
    setShowClientErrors(true);
    setSubmissionError(null);
    setBackendErrors({});

    if (hasClientErrors) {
      return;
    }

    try {
      const reportId = await createReportMutation.mutateAsync(draft);
      await submitReportMutation.mutateAsync(reportId);
      resetDraft();
      setShowClientErrors(false);
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 422) {
        const data = error.response.data as { errors?: Record<string, string[]> } | undefined;
        setBackendErrors(data?.errors ?? {});
        return;
      }
      setSubmissionError('Unable to submit report. Please try again.');
    }
  }, [createReportMutation, draft, hasClientErrors, resetDraft, submitReportMutation]);

  const isSaving = createReportMutation.isPending || submitReportMutation.isPending;

  const reportingPeriodStartErrors = [
    ...(draftValidation.formErrors.reporting_period_start ?? []),
    ...(backendErrors.reporting_period_start ?? [])
  ];
  const reportingPeriodEndErrors = [
    ...(draftValidation.formErrors.reporting_period_end ?? []),
    ...(backendErrors.reporting_period_end ?? [])
  ];
  const currencyErrors = [
    ...(draftValidation.formErrors.currency ?? []),
    ...(backendErrors.currency ?? [])
  ];
  const itemListErrors = [...(draftValidation.formErrors.items ?? []), ...(backendErrors.items ?? [])];

  return (
    <section className="employee-portal">
      <header>
        <h2>Submit new expense report</h2>
        <p>
          Draft expenses offline, attach receipts, and submit when ready. Policy validation runs before
          manager review.
        </p>
      </header>
      <div className="employee-portal__grid">
        <SummaryCard title="Open Reports" value="3" description="Awaiting submission" />
        <SummaryCard title="Pending Approvals" value="1" tone="warning" description="Manager review" />
        <SummaryCard title="Reimbursed YTD" value="$2,430" tone="success" />
      </div>
      <form
        className="employee-portal__form"
        onSubmit={(event) => {
          event.preventDefault();
          handleSubmit().catch(() => undefined);
        }}
      >
        <div className="employee-portal__form-grid">
          <div className="form-field">
            <label htmlFor="reportingPeriodStart">Period start</label>
            <input
              id="reportingPeriodStart"
              type="date"
              value={draft.reportingPeriodStart}
              onChange={(event) =>
                setDraft((current) => ({ ...current, reportingPeriodStart: event.target.value }))
              }
            />
            {(showClientErrors ? reportingPeriodStartErrors : backendErrors.reporting_period_start ?? [])
              .map((message) => (
                <p key={message} className="field-error">
                  {message}
                </p>
              ))}
          </div>
          <div className="form-field">
            <label htmlFor="reportingPeriodEnd">Period end</label>
            <input
              id="reportingPeriodEnd"
              type="date"
              value={draft.reportingPeriodEnd}
              onChange={(event) =>
                setDraft((current) => ({ ...current, reportingPeriodEnd: event.target.value }))
              }
            />
            {(showClientErrors ? reportingPeriodEndErrors : backendErrors.reporting_period_end ?? [])
              .map((message) => (
                <p key={message} className="field-error">
                  {message}
                </p>
              ))}
          </div>
          <div className="form-field">
            <label htmlFor="currency">Currency</label>
            <select
              id="currency"
              value={draft.currency}
              onChange={(event) => setDraft((current) => ({ ...current, currency: event.target.value }))}
            >
              <option value="USD">USD</option>
              <option value="EUR">EUR</option>
              <option value="CAD">CAD</option>
            </select>
            {(showClientErrors ? currencyErrors : backendErrors.currency ?? []).map((message) => (
              <p key={message} className="field-error">
                {message}
              </p>
            ))}
          </div>
        </div>
        <ExpenseItemsTable
          items={draft.items}
          onChange={(items) => setDraft((current) => ({ ...current, items }))}
          reportingPeriodStart={draft.reportingPeriodStart}
          reportingPeriodEnd={draft.reportingPeriodEnd}
          maxReceipts={RECEIPT_LIMIT.maxReceipts}
          maxReceiptBytes={RECEIPT_LIMIT.maxReceiptBytes}
          backendErrors={backendErrors}
        />
        {showClientErrors && itemListErrors.length > 0 && (
          <div className="employee-portal__validation-summary">
            {itemListErrors.map((message) => (
              <p key={message}>{message}</p>
            ))}
          </div>
        )}
        {submissionError && <p className="error">{submissionError}</p>}
        <button type="submit" disabled={isSaving}>
          {isSaving ? 'Submitting…' : 'Create and submit report'}
        </button>
      </form>
    </section>
  );
};

export default EmployeePortal;
