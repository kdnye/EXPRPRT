import './SummaryCard.css';

interface SummaryCardProps {
  title: string;
  value: string;
  description?: string;
  tone?: 'default' | 'success' | 'warning';
}

const SummaryCard = ({ title, value, description, tone = 'default' }: SummaryCardProps) => (
  <article className={`summary-card summary-card--${tone}`}>
    <h3>{title}</h3>
    <p className="summary-card__value">{value}</p>
    {description && <p className="summary-card__description">{description}</p>}
  </article>
);

export default SummaryCard;
