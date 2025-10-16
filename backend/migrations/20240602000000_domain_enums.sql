-- Define enum types for core domain fields
BEGIN;

CREATE TYPE employee_role AS ENUM (
    'employee',
    'manager',
    'finance',
    'admin'
);

CREATE TYPE report_status AS ENUM (
    'draft',
    'submitted',
    'manager_approved',
    'finance_finalized',
    'needs_changes',
    'denied'
);

CREATE TYPE approval_status AS ENUM (
    'approved',
    'denied',
    'needs_changes'
);

CREATE TYPE expense_category AS ENUM (
    'airfare',
    'lodging',
    'meal',
    'ground_transport',
    'mileage',
    'supplies',
    'other'
);

UPDATE employees
SET role = CASE
        WHEN role ILIKE 'employee' THEN 'employee'
        WHEN role ILIKE 'manager' THEN 'manager'
        WHEN role ILIKE 'finance' THEN 'finance'
        WHEN role ILIKE 'admin' THEN 'admin'
        ELSE role
    END;

UPDATE approvals
SET role = CASE
        WHEN role ILIKE 'employee' THEN 'employee'
        WHEN role ILIKE 'manager' THEN 'manager'
        WHEN role ILIKE 'finance' THEN 'finance'
        WHEN role ILIKE 'admin' THEN 'admin'
        ELSE role
    END,
    status = CASE
        WHEN status ILIKE 'approved' THEN 'approved'
        WHEN status ILIKE 'denied' THEN 'denied'
        WHEN status ILIKE 'needs_changes' THEN 'needs_changes'
        WHEN status ILIKE 'needs changes' THEN 'needs_changes'
        ELSE status
    END;

UPDATE expense_reports
SET status = CASE
        WHEN status ILIKE 'draft' THEN 'draft'
        WHEN status ILIKE 'submitted' THEN 'submitted'
        WHEN status ILIKE 'manager_approved' THEN 'manager_approved'
        WHEN status ILIKE 'managerapproved' THEN 'manager_approved'
        WHEN status ILIKE 'finance_finalized' THEN 'finance_finalized'
        WHEN status ILIKE 'financefinalized' THEN 'finance_finalized'
        WHEN status ILIKE 'needs_changes' THEN 'needs_changes'
        WHEN status ILIKE 'needs changes' THEN 'needs_changes'
        WHEN status ILIKE 'denied' THEN 'denied'
        ELSE status
    END;

UPDATE expense_items
SET category = CASE
        WHEN category ILIKE 'airfare' THEN 'airfare'
        WHEN category ILIKE 'lodging' THEN 'lodging'
        WHEN category ILIKE 'meal' THEN 'meal'
        WHEN category ILIKE 'ground_transport' THEN 'ground_transport'
        WHEN category ILIKE 'ground transport' THEN 'ground_transport'
        WHEN category ILIKE 'mileage' THEN 'mileage'
        WHEN category ILIKE 'supplies' THEN 'supplies'
        WHEN category ILIKE 'other' THEN 'other'
        ELSE category
    END;

UPDATE policy_caps
SET category = CASE
        WHEN category ILIKE 'airfare' THEN 'airfare'
        WHEN category ILIKE 'lodging' THEN 'lodging'
        WHEN category ILIKE 'meal' THEN 'meal'
        WHEN category ILIKE 'ground_transport' THEN 'ground_transport'
        WHEN category ILIKE 'ground transport' THEN 'ground_transport'
        WHEN category ILIKE 'mileage' THEN 'mileage'
        WHEN category ILIKE 'supplies' THEN 'supplies'
        WHEN category ILIKE 'other' THEN 'other'
        ELSE category
    END;

ALTER TABLE employees
    ALTER COLUMN role TYPE employee_role USING role::employee_role;

ALTER TABLE approvals
    ALTER COLUMN role TYPE employee_role USING role::employee_role,
    ALTER COLUMN status TYPE approval_status USING status::approval_status;

ALTER TABLE expense_reports
    ALTER COLUMN status TYPE report_status USING status::report_status;

ALTER TABLE expense_items
    ALTER COLUMN category TYPE expense_category USING category::expense_category;

ALTER TABLE policy_caps
    ALTER COLUMN category TYPE expense_category USING category::expense_category;

COMMIT;

-- Down
BEGIN;

ALTER TABLE approvals
    ALTER COLUMN role TYPE TEXT USING role::TEXT,
    ALTER COLUMN status TYPE TEXT USING status::TEXT;

ALTER TABLE employees
    ALTER COLUMN role TYPE TEXT USING role::TEXT;

ALTER TABLE expense_reports
    ALTER COLUMN status TYPE TEXT USING status::TEXT;

ALTER TABLE expense_items
    ALTER COLUMN category TYPE TEXT USING category::TEXT;

ALTER TABLE policy_caps
    ALTER COLUMN category TYPE TEXT USING category::TEXT;

DROP TYPE IF EXISTS expense_category;
DROP TYPE IF EXISTS approval_status;
DROP TYPE IF EXISTS report_status;
DROP TYPE IF EXISTS employee_role;

COMMIT;
