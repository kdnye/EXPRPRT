import { useQuery } from '@tanstack/react-query';
import { z } from 'zod';
import { request } from '../api/client';
import SummaryCard from '../components/SummaryCard';
import './FinanceConsole.css';

const financeBatchSchema = z.object({
  id: z.string(),
  reference: z.string(),
  totalReports: z.number(),
  totalAmount: z.string(),
  status: z.string(),
  exportedAt: z.string().optional()
});

const financeBatchResponseSchema = z.object({
  batches: z.array(financeBatchSchema)
});

type FinanceBatch = z.infer<typeof financeBatchSchema>;

const fetchBatches = async () => {
  const payload = await request<unknown>('get', '/finance/batches');
  return financeBatchResponseSchema.parse(payload).batches;
};

const FinanceConsole = () => {
  const { data = [], isLoading, isError } = useQuery({ queryKey: ['finance-batches'], queryFn: fetchBatches });

  return (
    <section className="finance-console">
      <header>
        <h2>Finance finalization</h2>
        <p>Batch approved reports, export to NetSuite, and monitor journal history with resilient retries.</p>
      </header>
      <div className="finance-console__grid">
        <SummaryCard title="Ready for export" value={isLoading ? '—' : String(data.length)} />
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
          {isError && (
            <tr>
              <td colSpan={5} className="finance-console__error">
                Unable to load finance batches.
              </td>
            </tr>
          )}
          {data.map((batch) => (
            <tr key={batch.id}>
              <td>{batch.reference}</td>
              <td>{batch.totalReports}</td>
              <td>{batch.totalAmount}</td>
              <td>{batch.status}</td>
              <td>{batch.exportedAt ?? 'Pending'}</td>
            </tr>
          ))}
          {data.length === 0 && !isLoading && (
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
