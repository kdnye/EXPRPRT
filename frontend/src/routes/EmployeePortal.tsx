import { useCallback } from 'react';
import { useMutation } from '@tanstack/react-query';
import { z } from 'zod';
import { request } from '../api/client';
import SummaryCard from '../components/SummaryCard';
import useExpenseDraft from '../hooks/useExpenseDraft';
import './EmployeePortal.css';

interface ExpenseDraft {
  reportingPeriodStart: string;
  reportingPeriodEnd: string;
  currency: string;
}

const createReportResponseSchema = z.object({
  report: z.object({
    id: z.string()
  })
});

const EXPENSE_DRAFT_ID = 'employee-portal:new-expense-report';

const EmployeePortal = () => {
  const createInitialDraft = useCallback<() => ExpenseDraft>(
    () => ({
      reportingPeriodStart: '',
      reportingPeriodEnd: '',
      currency: 'USD'
    }),
    []
  );

  const [draft, setDraft, resetDraft] = useExpenseDraft(EXPENSE_DRAFT_ID, createInitialDraft);

  const createReportMutation = useMutation({
    mutationFn: async (form: ExpenseDraft) => {
      const payload = await request<unknown>('post', '/expenses/reports', {
        reporting_period_start: form.reportingPeriodStart,
        reporting_period_end: form.reportingPeriodEnd,
        currency: form.currency
      });

      return createReportResponseSchema.parse(payload).report.id;
    }
  });

  const submitReportMutation = useMutation({
    mutationFn: (id: string) => request<unknown>('post', `/expenses/reports/${id}/submit`)
  });

  const handleSubmit = useCallback(async () => {
    const reportId = await createReportMutation.mutateAsync(draft);
    await submitReportMutation.mutateAsync(reportId);
    resetDraft();
  }, [createReportMutation, draft, resetDraft, submitReportMutation]);

  const isSaving = createReportMutation.isPending || submitReportMutation.isPending;
  const hasError = createReportMutation.isError || submitReportMutation.isError;
  const isFormValid = Boolean(draft.reportingPeriodStart && draft.reportingPeriodEnd && draft.currency);
  const hasDateError =
    draft.reportingPeriodStart !== '' &&
    draft.reportingPeriodEnd !== '' &&
    draft.reportingPeriodStart > draft.reportingPeriodEnd;

  return (
    <section className="employee-portal">
      <header>
        <h2>Submit new expense report</h2>
        <p>Draft expenses offline, attach receipts, and submit when ready. Policy validation runs before manager review.</p>
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
        <div className="form-row">
          <label htmlFor="reportingPeriodStart">Period start</label>
          <input
            id="reportingPeriodStart"
            type="date"
            value={draft.reportingPeriodStart}
            onChange={(event) =>
              setDraft((current) => ({ ...current, reportingPeriodStart: event.target.value }))
            }
            required
          />
        </div>
        <div className="form-row">
          <label htmlFor="reportingPeriodEnd">Period end</label>
          <input
            id="reportingPeriodEnd"
            type="date"
            value={draft.reportingPeriodEnd}
            onChange={(event) =>
              setDraft((current) => ({ ...current, reportingPeriodEnd: event.target.value }))
            }
            required
          />
          {hasDateError && <p className="field-error">End date must be on or after the start date.</p>}
        </div>
        <div className="form-row">
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
        </div>
        <button type="submit" disabled={isSaving || !isFormValid || hasDateError}>
          {isSaving ? 'Submitting…' : 'Create and submit report'}
        </button>
        {hasError && <p className="error">Unable to submit report. Please try again.</p>}
      </form>
    </section>
  );
};

export default EmployeePortal;
