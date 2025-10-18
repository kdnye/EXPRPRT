/**
 * Manager queue for reviewing employee-submitted reports with policy context.
 *
 * - Fetches the approval backlog from `GET /manager/queue` (see
 *   `backend/src/api/rest/expenses.rs` for submission and
 *   `backend/src/services/approvals.rs` for the manager decision engine).
 *   The queue aggregates reports waiting on a manager according to
 *   POLICY.md §"Approvals and Reimbursement Process".
 * - Highlights policy flags surfaced by the backend so managers can quickly
 *   identify exceptions before calling `POST /approvals/:id` in the action
 *   buttons that will be wired to `ApprovalService`.
 */
import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { z } from 'zod';
import { request } from '@/api/client';
import SummaryCard from '../components/SummaryCard';
import './ManagerConsole.css';

const managerQueueEntrySchema = z.object({
  report: z.object({
    id: z.string().uuid(),
    employeeId: z.string().uuid(),
    employeeHrIdentifier: z.string(),
    reportingPeriodStart: z.string(),
    reportingPeriodEnd: z.string(),
    submittedAt: z.string(),
    totalAmountCents: z.number(),
    totalReimbursableCents: z.number(),
    currency: z.string()
  }),
  lineItems: z.array(
    z.object({
      id: z.string().uuid(),
      reportId: z.string().uuid(),
      expenseDate: z.string(),
      category: z.string(),
      description: z.string().nullable(),
      amountCents: z.number(),
      reimbursable: z.boolean(),
      paymentMethod: z.string().nullable(),
      isPolicyException: z.boolean()
    })
  ),
  policyFlags: z.array(
    z.object({
      itemId: z.string().uuid(),
      category: z.string(),
      expenseDate: z.string(),
      description: z.string().nullable()
    })
  )
});

const managerQueueResponseSchema = z.object({
  queue: z.array(managerQueueEntrySchema)
});

type ManagerQueueEntry = z.infer<typeof managerQueueEntrySchema>;

const fetchQueue = async () => {
  const payload = await request<unknown>('get', '/manager/queue');
  return managerQueueResponseSchema.parse(payload).queue;
};

const formatCurrency = (amountCents: number, currency: string) =>
  new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(amountCents / 100);

const parseDate = (value: string) => (value.length <= 10 ? new Date(`${value}T00:00:00Z`) : new Date(value));

const formatDate = (isoDate: string) =>
  new Intl.DateTimeFormat(undefined, { year: 'numeric', month: 'short', day: 'numeric' }).format(parseDate(isoDate));

const ManagerConsole = () => {
  const {
    data = [],
    isLoading,
    isError
  } = useQuery<ManagerQueueEntry[]>({ queryKey: ['manager-queue'], queryFn: fetchQueue });

  const totalFlags = useMemo(
    () => data.reduce((sum, item) => sum + item.policyFlags.length, 0),
    [data]
  );

  const averageAgeDays = useMemo(() => {
    if (data.length === 0) {
      return null;
    }
    const now = Date.now();
    const totalDays = data.reduce((sum, item) => {
      const submitted = new Date(item.report.submittedAt).getTime();
      const ageMs = Math.max(now - submitted, 0);
      return sum + ageMs / (1000 * 60 * 60 * 24);
    }, 0);
    return totalDays / data.length;
  }, [data]);

  return (
    <section className="manager-console">
      <header>
        <h2>Manager approvals</h2>
        <p>Review submitted expense reports with policy highlights. Approval decisions sync instantly with finance.</p>
      </header>
      <div className="manager-console__grid">
        <SummaryCard title="Waiting for review" value={isLoading ? '—' : String(data.length)} />
        <SummaryCard title="Policy exceptions" value={isLoading ? '—' : String(totalFlags)} tone="warning" />
        <SummaryCard
          title="Average age"
          value={
            isLoading
              ? '—'
              : averageAgeDays === null
                ? '—'
                : `${averageAgeDays.toFixed(1)} day${averageAgeDays >= 2 ? 's' : ''}`
          }
        />
      </div>
      <div className="manager-console__list">
        {isError && <p className="manager-console__error">Unable to load pending reports.</p>}
        {isLoading && !isError && <p className="manager-console__loading">Loading queue…</p>}
        {!isLoading &&
          !isError &&
          data.map((item) => {
            const total = formatCurrency(item.report.totalAmountCents, item.report.currency);
            const reimbursable = formatCurrency(item.report.totalReimbursableCents, item.report.currency);
            const submitted = formatDate(item.report.submittedAt);
            const period = `${formatDate(item.report.reportingPeriodStart)} – ${formatDate(item.report.reportingPeriodEnd)}`;

            return (
              <article key={item.report.id}>
                <div>
                  <h3>{item.report.employeeHrIdentifier}</h3>
                  <p>
                    Reporting {period}
                    <br />
                    Submitted {submitted}
                  </p>
                </div>
                <div className="manager-console__list-meta">
                  <span>{total}</span>
                  <span className="manager-console__list-sub">Reimbursable {reimbursable}</span>
                  {item.policyFlags.length > 0 && (
                    <ul className="manager-console__list-flags">
                      {item.policyFlags.map((flag) => {
                        const flagDate = formatDate(flag.expenseDate);
                        const detail = flag.description ?? flag.category;
                        return (
                          <li key={flag.itemId}>
                            {flagDate}: {detail}
                          </li>
                        );
                      })}
                    </ul>
                  )}
                </div>
                <div className="manager-console__actions">
                  <button type="button">Approve</button>
                  <button type="button" className="secondary">
                    Request changes
                  </button>
                </div>
              </article>
            );
          })}
        {data.length === 0 && !isLoading && !isError && <p>No reports waiting for review.</p>}
      </div>
    </section>
  );
};

export default ManagerConsole;
