import { useQuery } from '@tanstack/react-query';
import { request } from '../api/client';
import SummaryCard from '../components/SummaryCard';
import './FinanceConsole.css';

interface FinanceBatch {
  id: string;
  reference: string;
  totalReports: number;
  totalAmount: string;
  status: string;
  exportedAt?: string;
}

const fetchBatches = async () => {
  const data = await request<{ batches: FinanceBatch[] }>('get', '/finance/batches');
  return data.batches;
};

const FinanceConsole = () => {
  const { data = [], isLoading } = useQuery({ queryKey: ['finance-batches'], queryFn: fetchBatches });

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
