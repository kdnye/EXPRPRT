import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { request } from '../api/client';
import SummaryCard from '../components/SummaryCard';
import './EmployeePortal.css';

interface ExpenseDraft {
  reportingPeriodStart: string;
  reportingPeriodEnd: string;
  currency: string;
}

const EmployeePortal = () => {
  const [draft, setDraft] = useState<ExpenseDraft>({
    reportingPeriodStart: '',
    reportingPeriodEnd: '',
    currency: 'USD'
  });

  const mutation = useMutation({
    mutationFn: () =>
      request<{ report: unknown }>('post', '/expenses/reports', {
        reporting_period_start: draft.reportingPeriodStart,
        reporting_period_end: draft.reportingPeriodEnd,
        currency: draft.currency
      })
  });

  const submitMutation = useMutation({
    mutationFn: (id: string) => request<{ report: unknown }>('post', `/expenses/reports/${id}/submit`)
  });

  const handleSubmit = async () => {
    const created = await mutation.mutateAsync();
    const id = (created.report as { id: string }).id;
    await submitMutation.mutateAsync(id);
  };

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
            onChange={(event) => setDraft((current) => ({ ...current, reportingPeriodStart: event.target.value }))}
            required
          />
        </div>
        <div className="form-row">
          <label htmlFor="reportingPeriodEnd">Period end</label>
          <input
            id="reportingPeriodEnd"
            type="date"
            value={draft.reportingPeriodEnd}
            onChange={(event) => setDraft((current) => ({ ...current, reportingPeriodEnd: event.target.value }))}
            required
          />
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
        <button type="submit" disabled={mutation.isPending || submitMutation.isPending}>
          {mutation.isPending || submitMutation.isPending ? 'Submitting…' : 'Create and submit report'}
        </button>
        {(mutation.isError || submitMutation.isError) && (
          <p className="error">Unable to submit report. Please try again.</p>
        )}
      </form>
    </section>
  );
};

export default EmployeePortal;
