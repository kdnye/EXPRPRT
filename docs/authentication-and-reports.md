# Authentication and Expense Report Submission

This guide summarizes how the developer login works and how expense reports move from draft to a submitted state. Use it while debugging local development issues.

## Login Flow

### Frontend (`frontend/src/components/LoginPrompt.tsx`)
- Collects the HR identifier and developer credential from the user.
- Sends a `POST /auth/login` request that includes `hr_identifier` and `credential`.
- Stores the returned JWT and role via the `useAuth` hook when the request succeeds.

### Backend (`backend/src/api/rest/auth.rs`)
- Compares the submitted credential with the configured `EXPENSES__AUTH__DEVELOPER_CREDENTIAL` (defaults to `dev-pass` in `.env.example`).
- Normalizes the HR identifier to uppercase and looks up the employee in the `employees` table.
- Issues a JWT signed with `EXPENSES__AUTH__JWT_SECRET` (default `dev-admin-secret`) when both checks pass.

### Login Troubleshooting Checklist
1. **Backend availability** – Confirm the Axum API is running on `http://localhost:8080` (or whatever `VITE_API_BASE` targets).
2. **Database state** – Ensure PostgreSQL is up and seeded. The seed migrations include sample employees such as `MGMT1001` and `EMP3101`.
3. **Credentials** – Use the seeded HR identifiers with the developer credential (`dev-pass`) unless you have changed `EXPENSES__AUTH__DEVELOPER_CREDENTIAL` in `.env`.
4. **Environment variables** – Double-check `.env` values for the developer credential and JWT secret when running in local mode.

If login still fails, inspect the network tab in your browser’s developer tools to review the request payload and any error response.

## Expense Report Submission Flow

### Frontend (`frontend/src/routes/EmployeePortal.tsx`)
- `handleSubmit` first posts the draft data to `POST /expenses/reports`.
- Converts dollar inputs to integer cents (`amount_cents`) before sending the payload.
- On success, calls `POST /expenses/reports/{reportId}/submit` to transition the report from `draft` to `submitted`.
- Clears the persisted draft via `resetDraft()` after the submission succeeds.

### Backend (`backend/src/services/expenses.rs`)
- `create_report` opens a transaction, inserts the `expense_reports` row with status `draft`, writes associated `expense_items`, and attaches receipts.
- `submit_report` updates the `expense_reports` status to `submitted` only when the current status is `draft` and the caller matches the `employee_id`.

### Submission Troubleshooting

| Symptom | Likely Source | What to Check |
| --- | --- | --- |
| Client-side validation prevents submission | Frontend | Verify required fields and business rules in `frontend/src/validation/expenseDraft.ts` and `frontend/src/routes/EmployeePortal.tsx`. The `handleSubmit` function exits early when `hasClientErrors` is true. |
| HTTP 422 with validation errors | Backend | Inspect `backend/src/api/rest/expenses.rs`, which calls `validate_create_report_payload`. Error messages point to the missing or invalid fields. |
| HTTP 409 conflict on `/submit` | Backend | The `submit_report` service only updates reports still in the `draft` state. Ensure the report has not already been submitted or modified elsewhere. |
| HTTP 401 unauthorized responses | Auth | Confirm the frontend includes the `Authorization: Bearer <token>` header (set automatically after login). Reauthenticate if the token expired or is missing. |

As with login issues, the network panel in your browser is the fastest way to confirm payloads, responses, and error codes during troubleshooting.
