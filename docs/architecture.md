# Expense Portal Architecture Blueprint

## Overview
This document translates the current manual expense workflow into a full-stack implementation using Rust, React, and PostgreSQL. The intent is to provide an actionable reference for engineering, product, and finance stakeholders while preserving policy guardrails from `POLICY.md` and the existing README.

## Goals & Non-Goals
- **Goals**
  - Digitize the employee → manager → finance workflow with auditable state transitions.
  - Enforce policy-driven validations (per diem, travel class, mileage) prior to manager review.
  - Support resilient receipt management, offline entry, and NetSuite journal exports.
  - Maintain extensible architecture for additional integrations and policy updates.
- **Non-Goals**
  - Real-time payroll disbursements (handled post-NetSuite).
  - Building a custom identity provider (integrate with corporate SSO later).
  - Replacing NetSuite; exporter focuses on journal entries required by finance.

## High-Level Architecture
```
┌────────────────┐      REST/GraphQL      ┌──────────────────────┐
│ React Frontend │◄──────────────────────►│ Rust Backend (Axum)  │
└────────────────┘                        └──────────────────────┘
        ▲                                           │
        │ Service worker sync                       │ Async Jobs (Tokio)
        │                                           ▼
┌────────────────┐      SQL / Storage API    ┌──────────────────────┐
│ IndexedDB/PWA  │◄────────────────────────►│ PostgreSQL + Storage │
└────────────────┘                           └──────────────────────┘
```
- **Frontend**: Vite + React single-page app with dual surfaces (employee portal and admin console). Service workers provide offline caching and background sync of drafts/uploads.
- **Backend**: Axum-based service implementing REST + GraphQL endpoints, orchestrating workflows, background jobs, and third-party integrations.
- **Database**: PostgreSQL schema for employees, expense reports, approvals, and NetSuite batches. Migrations managed via `sqlx-cli` or `refinery`.
- **Receipt Storage**: Pluggable provider selected via environment variables (local filesystem, S3, or GCS). Metadata persisted in PostgreSQL.

## Domain Model
| Table | Purpose | Key Fields |
|-------|---------|------------|
| `employees` | Directory synchronization for submitters and approvers. | `id (uuid)`, `hr_identifier`, `manager_id`, `department`, `is_manager`, `is_finance`, `policy_role_flags`, timestamps |
| `expense_reports` | Report header tracking workflow state. | `id`, `employee_id`, `reporting_period_start/end`, `status (draft/submitted/manager_approved/finance_finalized)`, `total_amount`, `total_reimbursable`, `currency`, `version` (for optimistic locking) |
| `expense_items` | Line-level entries mirroring spreadsheet columns. | `id`, `report_id`, `expense_date`, `category`, `gl_account_id`, `description`, `attendees`, `location`, `amount_cents`, `reimbursable`, `payment_method`, `is_policy_exception` |
| `receipts` | Receipt metadata and storage references. | `id`, `expense_item_id`, `file_key`, `file_name`, `mime_type`, `size_bytes`, `uploaded_by`, `virus_scan_status`, timestamps |
| `approvals` | Manager/finance decisions. | `id`, `report_id`, `approver_id`, `role (manager|finance)`, `status (approved|denied|needs_changes)`, `comments`, `policy_exception_notes`, timestamps |
| `netsuite_batches` | Finance finalization batches. | `id`, `batch_reference`, `finalized_by`, `finalized_at`, `status`, `export_job_id`, `exported_at`, `netsuite_response` |
| `journal_lines` | Journal entries prepared for NetSuite. | `id`, `batch_id`, `report_id`, `line_number`, `gl_account`, `amount_cents`, `department`, `class`, `memo`, `tax_code` |
| `mileage_rates` | Historical mileage reimbursements. | `effective_date`, `rate_cents_per_mile`, `source_reference` |
| `policy_caps` | Structured policy limits. | `id`, `policy_key`, `category`, `limit_type (per_diem|per_trip|per_day)`, `amount_cents`, `notes`, `active_from`, `active_to` |
| `audit_logs` | Tamper-resistant event trail. | `id`, `entity_type`, `entity_id`, `event_type`, `old_value`, `new_value`, `performed_by`, `performed_at`, `ip_address`, `user_agent`, `signature_hash` |
| `notifications` | Outgoing alert queue (email/Slack). | `id`, `channel`, `payload`, `status`, `retry_count`, `next_attempt_at` |

### Policy Automation Support
- Meal per-diem, mileage, and travel-class validation use `policy_caps` + category metadata.
- `expense_items.is_policy_exception` flips when validation fails; managers must provide override comments stored in `approvals.policy_exception_notes`.
- `audit_logs` capture any state change, including policy overrides, NetSuite responses, and receipt deletions.

## Backend Service Design
### Project Structure
```
backend/
  Cargo.toml
  src/
    main.rs
    api/
      mod.rs
      rest/
      graphql/
    domain/
      mod.rs
      models.rs
      policy.rs
    services/
      expenses.rs
      approvals.rs
      finance.rs
      notifications.rs
    infrastructure/
      db.rs
      storage/
      auth.rs
      netsuite.rs
      config.rs
    jobs/
      mod.rs
      exporter.rs
    validation/
      mod.rs
      rules.rs
    telemetry/
      logging.rs
      metrics.rs
```
- **Axum** for HTTP routing + middleware.
- **SQLx** for async PostgreSQL queries with compile-time checking.
- **SeaQuery** or Diesel is an alternative; SQLx chosen for async-first model aligning with Axum.
- **Tokio** tasks handle background jobs (NetSuite export, notifications, virus scanning callbacks).

### Authentication & Authorization
- JWT sessions issued after SSO callback (Auth0/Okta integration stubbed initially).
- Middleware extracts claims and maps to employee roles.
- Route guards enforce `manager`/`finance` scopes and check relationship (manager must own reportee).

### Validation Layer
- Central `validation::rules` module applying policy caps before persistence.
- Rules include:
  - Meal per-diem: compare aggregated meal amounts per day vs. `policy_caps`.
  - Mileage: compute `distance * rate` via `mileage_rates` history.
  - Travel class: reject business/first class unless flagged with justification.
  - Receipt requirements: enforce per-meal receipts and thresholds.
- Validation errors return structured responses with actionable guidance for the UI.

### Receipt Handling
- Chunked uploads via tus or S3 multipart; local dev uses filesystem backend.
- Virus scanning hook (ClamAV or third-party API) triggered post-upload; receipts remain pending until clean.
- Storage provider set by `RECEIPT_STORAGE_DRIVER` env (`local`, `s3`, `gcs`).
- Metadata persisted in `receipts`; `file_key` stores provider-specific identifier.

### Workflow Engine
- State machine encapsulated in `services::expenses::state_machine` ensuring valid transitions:
  - `draft` → `submitted` (employee submit, locks editing).
  - `submitted` → `manager_approved` / `needs_changes` / `denied`.
  - `manager_approved` → `finance_finalized` (finance may also push back to `needs_changes`).
- Optimistic locking via `version` field to prevent conflicting updates.
- Every transition writes to `audit_logs` with hashed signature for tamper evidence.

### Reporting & Search
- REST endpoints support pagination, filtering by date range, status, department, and policy flags.
- CSV/XLSX exports generated server-side using `calamine` or `xlsxwriter` library.
- Aggregated SQL views (`vw_expenses_by_employee`, `vw_expenses_by_category`, `vw_policy_exceptions`) back dashboards.

### NetSuite Integration
- Configured via `NETSUITE_*` environment variables stored in `.env`/secret manager.
- Export job groups finance-finalized reports into `netsuite_batches`.
- Each `journal_line` maps expense categories to GL accounts defined in policy tables.
- Job performs retry with exponential backoff on API failure; records response payload and status code for audit.
- Manual adjustments allowed before final transmit (via finance console) by editing pending `journal_lines`.

## Frontend Design
### Technology Choices
- **Vite + React + TypeScript** for fast development and typed components.
- **React Query** for server state management and offline caching; fallback to Redux Toolkit if complex client workflows emerge.
- **React Router** for portal vs. admin console navigation.
- **Component Library**: Tailwind CSS + Headless UI for accessible primitives; custom theming to match Freight Services branding.

### Employee Portal
- Spreadsheet-like grid for line items with keyboard shortcuts mirroring current XLSX template.
- Inline policy hints and validation messages sourced from backend responses.
- Receipt uploader with preview, progress bar, and offline queueing.
- Mileage calculator with Google Maps integration optional (feature-flagged) or manual entry with odometer fields.
- Draft autosave to IndexedDB via service worker background sync.

### Manager Console
- Inbox of pending reports, sortable by submission date and employee.
- Detailed report view with policy exception highlights and receipt thumbnails.
- Approve/Deny buttons requiring comments on overrides or denials.
- Bulk actions limited to marking multiple reports as "needs changes" to avoid accidental approvals.

### Finance Console
- Advanced filters (department, date range, policy exceptions).
- Batch builder to group approved reports and trigger NetSuite export.
- Export history with status, retry, and download of journal preview (CSV/XLSX).
- Audit timeline showing state changes and NetSuite responses.

### Offline & PWA Support
- Service worker caches shell assets, API responses, and queued mutations.
- Local `DraftStore` in IndexedDB storing pending reports and receipts; background sync pushes when online.
- Conflict resolution relies on report `version`; UI prompts user if server version diverges.

### Branding & Accessibility
- Typography: Bebas Neue for headings, Lato/Open Sans for body text.
- Color palette: primary blues (`#1C53A1`, `#428bca`, `#2d78e4`), accents `#00d084`, neutrals `#ffffff`, `#000000`.
- Include `fsi-logo (1).png` in header/sidebar; ensure alternative text for screen readers.
- WCAG AA contrast verified using tooling (Storybook a11y add-on planned).

## Workflow Automation & Notifications
- Daily digest job emails managers/finance about pending approvals using templated content.
- Slack notifications (optional) via webhook integration; payload redacts PII beyond employee name and report reference.
- Exception monitoring (Sentry/OpenTelemetry) captures validation errors, upload failures, and NetSuite responses.

## Infrastructure & DevOps
- **Docker Compose** for local stack: Rust API, Vite dev server, Postgres, storage emulator (MinIO), ClamAV.
- `.env.example` lists required variables (JWT secret, storage driver, NetSuite credentials, Slack webhook).
- **CI Pipeline** (GitHub Actions/CircleCI):
  1. `cargo fmt --check`, `cargo clippy -- -D warnings`.
  2. `cargo test` with Postgres service using migrations.
  3. `npm run lint`, `npm run test -- --watch=false`, `npm run typecheck`.
  4. Integration tests running against dockerized stack.
- **CD**: Container build and deploy to Kubernetes/Cloud Run; migrations run via job on deploy. Secrets managed through Vault or cloud secret manager.
- Observability: OpenTelemetry tracing, Prometheus metrics, structured JSON logs.

## Security & Compliance
- Sensitive data (receipts, notes) encrypted at rest (storage-level SSE + database column encryption where feasible).
- Access logging with IP/user agent; anomaly detection for unusual approval patterns.
- PII minimization: exports omit personal data unless finance explicitly opts in.
- Rate limiting + bot detection on auth endpoints.

## Implementation Roadmap
1. **Foundation (Sprint 1-2)**
   - Scaffold backend + frontend repos.
   - Establish Docker Compose, CI skeleton, basic auth, employee CRUD, and migrations.
2. **Expense Submission MVP (Sprint 3-4)**
   - Employee portal UI, receipt uploads, validation rules, draft vs. submit.
3. **Approvals Workflow (Sprint 5-6)**
   - Manager console, finance console skeleton, state machine integration, notifications.
4. **Policy Automation (Sprint 7)**
   - Implement caps, mileage calculations, exception logging.
5. **Reporting & NetSuite Export (Sprint 8-9)**
   - Build views, exports, NetSuite integration, batch management.
6. **Hardening & PWA (Sprint 10)**
   - Offline support polish, accessibility audit, penetration testing, performance tuning.
7. **Launch Preparation (Sprint 11)**
   - Documentation, training materials, production infrastructure, observability dashboards.

## Trade-Offs & Alternatives
- **Axum vs. Actix Web**: Axum chosen for modern async ergonomics and tower-based middleware. Actix offers higher throughput but adds complexity; can revisit if profiling shows Axum insufficient.
- **SQLx vs. Diesel**: SQLx provides async/await-friendly API and compile-time query checking without code generation. Diesel offers strong type safety but less async support. Given our async needs, SQLx is recommended.
- **React Query vs. Redux Toolkit**: React Query excels at server-state caching and optimistic updates required for offline use. Redux Toolkit could manage both server and UI state but introduces boilerplate. React Query combined with localized Zustand stores for UI state balances ergonomics and performance.
- **NetSuite Integration Timing**: Direct API integration ensures up-to-date journals but increases coupling. Alternative is CSV upload; we retain CSV export as manual fallback while automating API calls when credentials exist.

## Open Questions
- Confirm SSO provider and timeline for integrating JWT issuance.
- Determine retention policy for receipts and audit logs (GDPR/finance requirements).
- Decide on mileage source of truth (Google Maps API vs. manual entry with audits).
- Clarify finance approval SLA for notification escalation rules.

## Next Steps
- Review schema with finance and HR to validate fields and policy coverage.
- Prototype receipt upload flow with local storage backend to assess UX.
- Define NetSuite account mapping table structure based on POLICY schedule.
- Draft ADRs for authentication provider choice and storage backend selection once constraints known.
