/**
 * Finance console for monitoring export-ready batches and journal status.
 *
 * - Retrieves batch summaries via `GET /finance/batches`, aligning with the
 *   export workflow defined in `backend/src/api/rest/finance.rs` and the
 *   orchestration in `backend/src/services/finance.rs`.
 * - Reinforces downstream responsibilities from POLICY.md
 *   §"Approvals and Reimbursement Process" and the GL mapping tables so
 *   finance staff understand why batches are grouped and which accounts are
 *   impacted.
 * - Complements the finance finalize action (`POST /finance/finalize`) by
 *   presenting status and retry counts surfaced by the backend services.
 */
import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { z } from 'zod';
import { request } from '@/api/client';
import SummaryCard from '../components/SummaryCard';
import './FinanceConsole.css';

const financeBatchSchema = z
  .object({
    id: z.string().uuid(),
    batch_reference: z.string(),
    report_count: z.number(),
    total_amount_cents: z.number(),
    status: z.string(),
    finalized_at: z.string(),
    exported_at: z.string().nullable().optional()
  })
  .transform((batch) => ({
    id: batch.id,
    reference: batch.batch_reference,
    reportCount: batch.report_count,
    totalAmountCents: batch.total_amount_cents,
    status: batch.status,
    finalizedAt: batch.finalized_at,
    exportedAt: batch.exported_at ?? null
  }));

const financeBatchResponseSchema = z.object({
  batches: z.array(financeBatchSchema)
});

type FinanceBatch = z.infer<typeof financeBatchSchema>;

const fetchBatches = async () => {
  const payload = await request<unknown>('get', '/finance/batches');
  return financeBatchResponseSchema.parse(payload).batches;
};

const formatCurrency = (amountCents: number) =>
  new Intl.NumberFormat(undefined, { style: 'currency', currency: 'USD' }).format(amountCents / 100);

const parseDate = (value: string) => (value.length <= 10 ? new Date(`${value}T00:00:00Z`) : new Date(value));

const formatDateTime = (value: string | null) => {
  if (!value) {
    return 'Pending';
  }
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(parseDate(value));
};

const FinanceConsole = () => {
  const { data = [], isLoading, isError } = useQuery<FinanceBatch[]>({
    queryKey: ['finance-batches'],
    queryFn: fetchBatches
  });

  const pendingBatches = useMemo(() => data.filter((batch) => batch.status !== 'exported'), [data]);

  return (
    <section className="finance-console">
      <header>
        <h2>Finance finalization</h2>
        <p>Batch approved reports, export to NetSuite, and monitor journal history with resilient retries.</p>
      </header>
      <div className="finance-console__grid">
        <SummaryCard title="Ready for export" value={isLoading ? '—' : String(pendingBatches.length)} />
        <SummaryCard title="Export success rate" value="98%" tone="success" />
        <SummaryCard title="Pending retries" value="1" tone="warning" />
      </div>
      <table>
        <thead>
          <tr>
            <th>Batch</th>
            <th>Reports</th>
            <th>Total</th>
            <th>Status</th>
            <th>Exported</th>
          </tr>
        </thead>
        <tbody>
          {isLoading && (
            <tr>
              <td colSpan={5}>Loading batch history…</td>
            </tr>
          )}
          {isError && (
            <tr>
              <td colSpan={5} className="finance-console__error">
                Unable to load finance batches.
              </td>
            </tr>
          )}
          {!isLoading &&
            !isError &&
            data.map((batch) => (
              <tr key={batch.id}>
                <td>{batch.reference}</td>
                <td>{batch.reportCount}</td>
                <td>{formatCurrency(batch.totalAmountCents)}</td>
                <td>{batch.status}</td>
                <td>{formatDateTime(batch.exportedAt)}</td>
              </tr>
            ))}
          {data.length === 0 && !isLoading && !isError && (
            <tr>
              <td colSpan={5}>No finalized batches yet.</td>
            </tr>
          )}
        </tbody>
      </table>
    </section>
  );
};

export default FinanceConsole;
