import './StatusPill.css';

type Status = 'online' | 'offline' | 'syncing';

const STATUS_COPY: Record<Status, string> = {
  online: 'Online',
  offline: 'Offline',
  syncing: 'Syncing'
};

const StatusPill = ({ status }: { status: Status }) => {
  return <span className={`status-pill status-pill--${status}`}>{STATUS_COPY[status]}</span>;
};

export default StatusPill;
