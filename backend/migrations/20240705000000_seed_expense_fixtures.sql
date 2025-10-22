-- Seed sample expense data to showcase line-item breakdowns in manager and finance views.
INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000201',
    'MGMT1001',
    NULL,
    'Operations',
    'manager',
    NOW()
)
ON CONFLICT (id) DO UPDATE SET department = EXCLUDED.department;

INSERT INTO employees (id, hr_identifier, manager_id, department, role, created_at)
VALUES (
    '00000000-0000-0000-0000-000000000301',
    'EMP3101',
    '00000000-0000-0000-0000-000000000201',
    'Logistics',
    'employee',
    NOW()
)
ON CONFLICT (id) DO UPDATE SET manager_id = EXCLUDED.manager_id, department = EXCLUDED.department;

INSERT INTO expense_reports (
    id,
    employee_id,
    reporting_period_start,
    reporting_period_end,
    status,
    total_amount_cents,
    total_reimbursable_cents,
    currency,
    version,
    created_at,
    updated_at
)
VALUES (
    '00000000-0000-0000-0000-000000000401',
    '00000000-0000-0000-0000-000000000301',
    DATE '2024-04-01',
    DATE '2024-04-30',
    'submitted',
    68500,
    48500,
    'USD',
    2,
    NOW(),
    NOW()
)
ON CONFLICT (id) DO UPDATE SET
    total_amount_cents = EXCLUDED.total_amount_cents,
    total_reimbursable_cents = EXCLUDED.total_reimbursable_cents,
    status = EXCLUDED.status,
    updated_at = NOW();

INSERT INTO expense_items (
    id,
    report_id,
    expense_date,
    category,
    gl_account_id,
    description,
    attendees,
    location,
    amount_cents,
    reimbursable,
    payment_method,
    is_policy_exception
)
SELECT item_id,
       report_id,
       expense_date,
       category,
       gl_account_id,
       description,
       attendees,
       location,
       amount_cents,
       reimbursable,
       payment_method,
       is_policy_exception
FROM (
    SELECT
        '00000000-0000-0000-0000-000000000501'::uuid AS item_id,
        '00000000-0000-0000-0000-000000000401'::uuid AS report_id,
        DATE '2024-04-05' AS expense_date,
        'meal' AS category,
        NULL::uuid AS gl_account_id,
        'Onsite workshop lunch' AS description,
        'Client Ops team' AS attendees,
        'Denver, CO' AS location,
        18500 AS amount_cents,
        TRUE AS reimbursable,
        'corporate_card' AS payment_method,
        FALSE AS is_policy_exception
    UNION ALL
    SELECT
        '00000000-0000-0000-0000-000000000502'::uuid,
        '00000000-0000-0000-0000-000000000401'::uuid,
        DATE '2024-04-07',
        'lodging',
        NULL::uuid,
        'Hotel - client onsite',
        NULL::text,
        'Denver, CO',
        30000,
        TRUE,
        'corporate_card',
        FALSE
    UNION ALL
    SELECT
        '00000000-0000-0000-0000-000000000503'::uuid,
        '00000000-0000-0000-0000-000000000401'::uuid,
        DATE '2024-04-08',
        'ground_transport',
        NULL::uuid,
        'Freight yard shuttle',
        NULL::text,
        'Denver, CO',
        20000,
        FALSE,
        'personal_card',
        FALSE
) items
ON CONFLICT (id) DO UPDATE SET
    expense_date = EXCLUDED.expense_date,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    attendees = EXCLUDED.attendees,
    location = EXCLUDED.location,
    amount_cents = EXCLUDED.amount_cents,
    reimbursable = EXCLUDED.reimbursable,
    payment_method = EXCLUDED.payment_method,
    is_policy_exception = EXCLUDED.is_policy_exception;

INSERT INTO receipts (
    id,
    expense_item_id,
    file_key,
    file_name,
    mime_type,
    size_bytes,
    uploaded_by
)
VALUES
    (
        '00000000-0000-0000-0000-000000000601',
        '00000000-0000-0000-0000-000000000501',
        'seed/receipts/lunch-2024-04-05.pdf',
        'lunch-2024-04-05.pdf',
        'application/pdf',
        45210,
        '00000000-0000-0000-0000-000000000301'
    ),
    (
        '00000000-0000-0000-0000-000000000602',
        '00000000-0000-0000-0000-000000000502',
        'seed/receipts/hotel-2024-04-07.pdf',
        'hotel-2024-04-07.pdf',
        'application/pdf',
        78200,
        '00000000-0000-0000-0000-000000000301'
    )
ON CONFLICT (id) DO UPDATE SET
    file_key = EXCLUDED.file_key,
    file_name = EXCLUDED.file_name,
    mime_type = EXCLUDED.mime_type,
    size_bytes = EXCLUDED.size_bytes,
    uploaded_by = EXCLUDED.uploaded_by;
