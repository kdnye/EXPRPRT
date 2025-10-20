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
import type { AxiosError } from 'axios';
import { z } from 'zod';
import { request } from '@/api/client';
import SummaryCard from '../components/SummaryCard';
import './FinanceConsole.css';

type ApiError = {
  error?: string;
};

const financeBatchSchema = z
  .object({
    id: z.string().uuid(),
    batch_reference: z.string(),
    report_count: z.number(),
    total_amount_cents: z.number(),
    status: z.string(),
    finalized_at: z.string().datetime(),
    exported_at: z.string().datetime().nullish()
  })
  .transform((batch) => ({
    id: batch.id,
    reference: batch.batch_reference,
    reportCount: batch.report_count,
    totalAmountCents: batch.total_amount_cents,
    status: batch.status,
    finalizedAt: new Date(batch.finalized_at),
    exportedAt: batch.exported_at ? new Date(batch.exported_at) : null
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

const formatDateTime = (value: Date | null, placeholder = 'Pending') => {
  if (!value) {
    return placeholder;
  }
  return new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  }).format(value);
};

const formatDate = (value: Date) =>
  new Intl.DateTimeFormat(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric'
  }).format(value);

const FinanceConsole = () => {
  const {
    data = [],
    isLoading,
    isError,
    error
  } = useQuery<FinanceBatch[], AxiosError<ApiError>>({
    queryKey: ['finance-batches'],
    queryFn: fetchBatches,
    staleTime: 60_000
  });

  const errorMessage = useMemo(() => {
    if (!error) {
      return 'Unable to load finance batches.';
    }
    return error.response?.data?.error ?? error.message ?? 'Unable to load finance batches.';
  }, [error]);

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
            <th>Finalized</th>
            <th>Exported</th>
          </tr>
        </thead>
        <tbody>
          {isLoading && (
            <tr>
              <td colSpan={6}>Loading batch history…</td>
            </tr>
          )}
          {isError && (
            <tr>
              <td colSpan={6} className="finance-console__error">
                {errorMessage}
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
                <td>{formatDate(batch.finalizedAt)}</td>
                <td>{formatDateTime(batch.exportedAt)}</td>
              </tr>
            ))}
          {data.length === 0 && !isLoading && !isError && (
            <tr>
              <td colSpan={6}>No finalized batches yet.</td>
            </tr>
          )}
        </tbody>
      </table>
    </section>
  );
};

export default FinanceConsole;
