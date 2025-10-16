# Expense Reimbursement Policy

The Company reimburses employees for reasonable and necessary expenses incurred while conducting business on the Company's behalf. These guidelines are enforced programmatically in [`backend/src/domain/policy.rs`](backend/src/domain/policy.rs) and [`backend/src/services/expenses.rs`](backend/src/services/expenses.rs), with UI guardrails in the expense submission flows under [`frontend/src/routes`](frontend/src/routes).

## Reimbursable Expenses

### Travel

#### Domestic Airfare

- Book the lowest available coach fare.
- First-class tickets and voluntary upgrades are not reimbursable.

#### International Airfare

- Business class is permitted only when a single continuous published flight segment exceeds eight (8) hours (layovers excluded).
- For shorter itineraries, book coach class.

#### Unused, Non-Refundable, or Lost Tickets

- Request refunds for unused tickets immediately; apply non-refundable balances to future trips.
- Freight Services will assist with change fees when schedule adjustments are required—coordinate with your Manager.
- Lost airline tickets are not reimbursed. Contact Freight Services for replacement tickets and, if necessary, submit a lost ticket application.

#### Miscellaneous Travel Charges

- Reasonable phone charges are reimbursable. Prefer calling cards or mobile phones to hotel room lines; use calling cards internationally.
- Hotel gym fees up to **$15 per day** are reimbursable.

### Meals

- Submit itemized receipts for every meal, regardless of cost.
- When a receipt is unavailable, reimbursement is limited to the following per diem amounts:

| Meal      | Maximum Reimbursement |
|-----------|-----------------------|
| Breakfast | $10                   |
| Lunch     | $15                   |
| Dinner    | $25                   |

- Use the amounts above as planning guidelines. Higher costs may be approved for major metropolitan areas (e.g., New York City, Chicago).

### Entertainment

- Business-related entertainment is reimbursable when it directly supports Company business (e.g., client meals with documented discussions).
- Personal entertainment is not reimbursable.

### Car Rentals

- **Vehicle size:** Rent compact cars unless medical needs require a larger vehicle; two or more employees traveling together may rent a mid-size vehicle.
- **Insurance:** In the United States and Canada, purchase the rental company's loss damage waiver, personal effects protection, and personal accident coverage. Freight Services also maintains American Express coverage (for vehicles valued at $50,000 or less). Consult the Company if unsure about required coverage.
- Arrange international rentals prior to departure in consultation with your Manager.

### Other Transportation

- **Mileage reimbursement:** Claim the IRS mileage rate for business mileage exceeding your normal round-trip commute. Commuting time and miles are not reimbursable.
- **Parking, tolls, and ground transport:** Submit receipts for parking, bridge, tunnel, and road tolls. Use shuttles and taxis when practical; obtain Manager approval before booking limousines or town cars.
- **Long-term travel:** For trips lasting thirty-six (36) hours or more, use airport long-term or off-site parking.

### Miscellaneous Travel

- **Laundry:** Laundry, dry cleaning, and pressing are reimbursable when the trip exceeds seven (7) full days.
- **Gratuities:** Reasonable tips for travel-related services (e.g., meals, shuttles) are reimbursable.

### Cancellations

- Notify hotels, rental agencies, airlines, and other vendors of cancellations before the cutoff to avoid no-show fees.
- No-show fees are not reimbursable unless the charges were genuinely unavoidable.

## Non-Reimbursable Expenses

The following non-inclusive list of expenses is never reimbursed:

- Airphone charges except in emergencies, with written justification.
- Annual fees, interest, or late charges on personal credit cards.
- Personal sundries (reading materials, medication, batteries, toiletries, etc.).
- Personal grooming (haircuts, manicures, similar services).
- Personal entertainment (movies, videos, airline headphones, and similar non-business items).
- Airline club dues (e.g., Red Carpet Club).
- Travel accident insurance unless bundled with required rental car coverage.
- Traffic or parking violation fines.
- Theft of personal property, including items stolen from personal or rental vehicles, unless covered by rental insurance.

## Approvals and Reimbursement Process

- Obtain written approval for business expenses from the Vice President of Operations.
- Submit approved expense reports with dated receipts to the Vice President of Operations at month-end for the period in which expenses were incurred.
- Only the Vice President of Operations may authorize policy exceptions.
- Expense report forms are available from your Manager.
- Reimbursement checks are issued promptly after approval. Expense reimbursements comply with applicable law and are not treated as compensation.

## General Ledger Mapping

The GL mappings below align with the domain expense models in [`backend/src/domain/models.rs`](backend/src/domain/models.rs) and are consumed by the finance integration services described in [`backend/src/services/expenses.rs`](backend/src/services/expenses.rs).

### Expense Type to GL Account

| Expense Type                        | GL Account |
|-------------------------------------|------------|
| Maintenance & Repairs               | 51020      |
| Parking & Storage - COGS            | 51070      |
| Vehicle Supplies                    | 51090      |
| State Permits/Fees/Tolls            | 52030      |
| Meals & Entertainment - COGS        | 52070      |
| Travel - COGS                       | 52080      |
| FSI Global Overhead                 | 56000      |
| Telephone - GA                      | 62000      |
| Utilities                           | 62070      |
| IT/Computer                         | 62080      |
| Office Supplies                     | 62090      |
| Printing & Postage                  | 62100      |
| Meals & Entertainment - GA          | 64180      |
| Travel - GA                         | 64190      |
| FSI Global G&A                      | 66500      |

_Table 1: Expense type mapping used by finance posting services._

### Financial Statement Category Mapping

| Description                        | Account # |
|------------------------------------|-----------|
| Cost of Goods Sold                 | 50000     |
| OPS Wages                          | 50500     |
| OPS Wages - Terminal               | 50510     |
| OPS Wages - Driver                 | 50520     |
| OPS Wages - Bonus                  | 50540     |
| OPS Wages - Training               | 50550     |
| OPS Wages - Overtime               | 50560     |
| OPS Wages - PTO                    | 50570     |
| OPS Wages - Holiday                | 50580     |
| Workers Compensation               | 50590     |
| Workers Comp - Safety Incentive    | 50595     |
| Purchase Transportation - Agent    | 50600     |
| Purchase Transport - Carrier       | 50610     |
| Purchase Transport - Small Pack    | 50620     |
| Vehicle Expense                    | 51000     |
| Vehicle - Fuel                     | 51010     |
| Vehicle - Maint/Repairs            | 51020     |
| Vehicle - Leased                   | 51030     |
| Vehicle - License/Registration     | 51040     |
| Vehicle - Insurance                | 51050     |
| Vehicle - Tracking                 | 51060     |
| Vehicle - Parking/Storage          | 51070     |
| Vehicle - Equipment                | 51080     |
| Vehicle - Supplies                 | 51090     |
| Tax - IFTA                         | 52010     |
| Tax - Road Use                     | 52020     |
| State Permits/Fees                 | 52030     |
| Equipment Rental                   | 52040     |
| Uniforms                           | 52050     |
| Terminal Supplies                  | 52060     |
| Meals/Entertainment                | 52070     |
| Travel Expense                     | 52080     |
| G&A Expense                        | 60000     |
| GA Wages                           | 60100     |
| GA Wages - Admin                   | 60110     |
| GA Wages - Bonus                   | 60115     |
| GA Wages - Overtime                | 60120     |
| GA Wages - PTO                     | 60130     |
| GA Wages - Holiday                 | 60140     |
| Worker's Compensation              | 60150     |
| (Unassigned)                       | 61000     |
| Insurance - Business               | 61010     |
| Insurance - Cargo                  | 61020     |
| Insurance - Auto                   | 61030     |
| Insurance - Worker's Comp          | 61040     |
| Fringe                             | 61500     |
| Fringe - Health Insurance (ER)     | 61505     |
| Fringe - Health Insurance (EE)     | 61510     |
| (Unassigned)                       | 61515     |
| Fringe - Payroll Taxes             | 61520     |
| Fringe - 401K                      | 61530     |
| Employment Screening               | 61540     |
| Employee Training                  | 61550     |
| Telephone                          | 62000     |
| Rent - Office                      | 62010     |
| Rent - Offsite                     | 62020     |
| Payroll Service Fees               | 62025     |
| Professional Fees - Legal          | 62030     |
| Professional Fees - Accounting     | 62040     |
| Professional Fees - Contractor     | 62050     |
| Facility Maintenance/Repairs       | 62060     |
| Utilities                          | 62070     |
| IT/Computer                        | 62080     |
| Office Supplies                    | 62090     |
| Printing/Postage                   | 62100     |
| Property Taxes                     | 62110     |
| Licenses/Permits                   | 62120     |
| BD Wages                           | 64100     |
| BD Wages - Business Development    | 64110     |
| BD Wages - Bonus                   | 64115     |
| BD Wages - Overtime                | 64120     |
| BD Wages - PTO                     | 64130     |
| BD Wages - Holiday                 | 64140     |
| Recruiting                         | 64150     |
| Advertising/Promotion              | 64160     |
| Dues/Subscriptions                 | 64170     |
| Charitable Contributions           | 64175     |
| Meals/Entertainment                | 64180     |

_Table 2: Financial statement roll-up supporting domain model validations and finance exports._
