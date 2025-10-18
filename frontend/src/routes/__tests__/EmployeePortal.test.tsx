import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { vi } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import EmployeePortal from '../EmployeePortal';
import { request } from '@/api/client';

vi.mock('@/api/client', () => ({
  request: vi.fn()
}));

describe('EmployeePortal', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it('persists edited expense rows to the draft store', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <EmployeePortal />
      </QueryClientProvider>
    );

    await act(async () => {
      await user.click(screen.getByRole('button', { name: /add expense/i }));
    });

    const dateInput = screen.getByLabelText('Expense date');
    await act(async () => {
      fireEvent.change(dateInput, { target: { value: '2024-05-03' } });
    });

    const amountInput = screen.getByLabelText('Expense amount');
    await user.clear(amountInput);
    await user.type(amountInput, '123.45');

    await act(async () => {
      vi.advanceTimersByTime(500);
    });

    const draftsRaw = localStorage.getItem('fsi-expense-drafts');
    expect(draftsRaw).toBeTruthy();
    const drafts = JSON.parse(draftsRaw ?? '[]');
    expect(drafts).toHaveLength(1);
    expect(drafts[0].payload.items[0]).toMatchObject({
      expenseDate: '2024-05-03',
      amount: '123.45'
    });
  });

  it('submits a populated draft and clears stored state', async () => {
    const requestMock = request as unknown as vi.Mock;
    requestMock.mockImplementation((method: string, url: string) => {
      if (url === '/expenses/reports') {
        return Promise.resolve({ report: { id: 'report-123' } });
      }
      return Promise.resolve({});
    });

    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const queryClient = new QueryClient();
    render(
      <QueryClientProvider client={queryClient}>
        <EmployeePortal />
      </QueryClientProvider>
    );

    await act(async () => {
      fireEvent.change(screen.getByLabelText('Period start'), {
        target: { value: '2024-05-01' }
      });
    });
    await act(async () => {
      fireEvent.change(screen.getByLabelText('Period end'), {
        target: { value: '2024-05-10' }
      });
    });

    await act(async () => {
      await user.click(screen.getByRole('button', { name: /add expense/i }));
    });

    const dateInput = screen.getByLabelText('Expense date');
    await act(async () => {
      fireEvent.change(dateInput, { target: { value: '2024-05-03' } });
    });

    const amountInput = screen.getByLabelText('Expense amount');
    await user.clear(amountInput);
    await user.type(amountInput, '200.00');

    await act(async () => {
      vi.advanceTimersByTime(500);
    });

    await act(async () => {
      await user.click(screen.getByRole('button', { name: /create and submit report/i }));
    });

    await act(async () => {
      vi.runAllTimers();
    });

    expect(requestMock).toHaveBeenCalledTimes(2);

    const [createMethod, createUrl, createPayload] = requestMock.mock.calls[0];
    expect(createMethod).toBe('post');
    expect(createUrl).toBe('/expenses/reports');
    expect(createPayload.items).toHaveLength(1);
    expect(createPayload.items[0]).toMatchObject({
      expense_date: '2024-05-03',
      amount_cents: 20000,
      reimbursable: true
    });

    await act(async () => {
      vi.advanceTimersByTime(500);
    });

    expect(localStorage.getItem('fsi-expense-drafts')).toBe('[]');
  });
});
