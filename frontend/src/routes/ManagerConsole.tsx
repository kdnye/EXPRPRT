import { useQuery } from '@tanstack/react-query';
import { z } from 'zod';
import { request } from '../api/client';
import SummaryCard from '../components/SummaryCard';
import './ManagerConsole.css';

const managerQueueItemSchema = z.object({
  id: z.string(),
  employee: z.string(),
  submittedAt: z.string(),
  total: z.string(),
  policyFlags: z.array(z.string())
});

const managerQueueResponseSchema = z.object({
  queue: z.array(managerQueueItemSchema)
});

type ManagerQueueItem = z.infer<typeof managerQueueItemSchema>;

const fetchQueue = async () => {
  const payload = await request<unknown>('get', '/manager/queue');
  return managerQueueResponseSchema.parse(payload).queue;
};

const ManagerConsole = () => {
  const { data = [], isLoading, isError } = useQuery({ queryKey: ['manager-queue'], queryFn: fetchQueue });

  return (
    <section className="manager-console">
      <header>
        <h2>Manager approvals</h2>
        <p>Review submitted expense reports with policy highlights. Approval decisions sync instantly with finance.</p>
      </header>
      <div className="manager-console__grid">
        <SummaryCard title="Waiting for review" value={isLoading ? '—' : String(data.length)} />
        <SummaryCard title="Policy exceptions" value="2" tone="warning" />
        <SummaryCard title="Average age" value="1.6 days" />
      </div>
      <div className="manager-console__list">
        {isError && <p className="manager-console__error">Unable to load pending reports.</p>}
        {data.map((item) => (
          <article key={item.id}>
            <div>
              <h3>{item.employee}</h3>
              <p>Submitted {item.submittedAt}</p>
            </div>
            <div className="manager-console__list-meta">
              <span>{item.total}</span>
              {item.policyFlags.length > 0 && <span className="flags">{item.policyFlags.join(', ')}</span>}
            </div>
            <div className="manager-console__actions">
              <button type="button">Approve</button>
              <button type="button" className="secondary">
                Request changes
              </button>
            </div>
          </article>
        ))}
        {data.length === 0 && !isLoading && <p>No reports waiting for review.</p>}
      </div>
    </section>
  );
};

export default ManagerConsole;
