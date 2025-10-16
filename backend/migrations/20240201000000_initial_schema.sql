-- Initial schema for Freight Services expense portal
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE employees (
    id UUID PRIMARY KEY,
    hr_identifier TEXT NOT NULL UNIQUE,
    manager_id UUID REFERENCES employees(id),
    department TEXT,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE expense_reports (
    id UUID PRIMARY KEY,
    employee_id UUID NOT NULL REFERENCES employees(id),
    reporting_period_start DATE NOT NULL,
    reporting_period_end DATE NOT NULL,
    status TEXT NOT NULL,
    total_amount_cents BIGINT NOT NULL DEFAULT 0,
    total_reimbursable_cents BIGINT NOT NULL DEFAULT 0,
    currency TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE expense_items (
    id UUID PRIMARY KEY,
    report_id UUID NOT NULL REFERENCES expense_reports(id) ON DELETE CASCADE,
    expense_date DATE NOT NULL,
    category TEXT NOT NULL,
    gl_account_id UUID,
    description TEXT,
    attendees TEXT,
    location TEXT,
    amount_cents BIGINT NOT NULL,
    reimbursable BOOLEAN NOT NULL DEFAULT TRUE,
    payment_method TEXT,
    is_policy_exception BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE receipts (
    id UUID PRIMARY KEY,
    expense_item_id UUID NOT NULL REFERENCES expense_items(id) ON DELETE CASCADE,
    file_key TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    uploaded_by UUID NOT NULL REFERENCES employees(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE approvals (
    id UUID PRIMARY KEY,
    report_id UUID NOT NULL REFERENCES expense_reports(id) ON DELETE CASCADE,
    approver_id UUID NOT NULL REFERENCES employees(id),
    role TEXT NOT NULL,
    status TEXT NOT NULL,
    comments TEXT,
    policy_exception_notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE netsuite_batches (
    id UUID PRIMARY KEY,
    batch_reference TEXT NOT NULL,
    finalized_by UUID NOT NULL REFERENCES employees(id),
    finalized_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL,
    exported_at TIMESTAMPTZ,
    netsuite_response JSONB
);

CREATE TABLE journal_lines (
    id UUID PRIMARY KEY,
    batch_id UUID NOT NULL REFERENCES netsuite_batches(id) ON DELETE CASCADE,
    report_id UUID NOT NULL REFERENCES expense_reports(id),
    line_number INTEGER NOT NULL,
    gl_account TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    department TEXT,
    class TEXT,
    memo TEXT,
    tax_code TEXT
);

CREATE TABLE mileage_rates (
    id UUID PRIMARY KEY,
    effective_date DATE NOT NULL,
    rate_cents_per_mile INTEGER NOT NULL,
    source_reference TEXT
);

CREATE TABLE policy_caps (
    id UUID PRIMARY KEY,
    policy_key TEXT NOT NULL,
    category TEXT NOT NULL,
    limit_type TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    notes TEXT,
    active_from DATE NOT NULL,
    active_to DATE
);

CREATE TABLE audit_logs (
    id UUID PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    old_value JSONB,
    new_value JSONB,
    performed_by UUID REFERENCES employees(id),
    performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip_address TEXT,
    user_agent TEXT,
    signature_hash TEXT NOT NULL
);

CREATE INDEX idx_expense_reports_employee ON expense_reports(employee_id);
CREATE INDEX idx_expense_items_report ON expense_items(report_id);
CREATE INDEX idx_receipts_item ON receipts(expense_item_id);
CREATE INDEX idx_approvals_report ON approvals(report_id);
CREATE INDEX idx_journal_lines_batch ON journal_lines(batch_id);
CREATE INDEX idx_policy_caps_category ON policy_caps(category);
