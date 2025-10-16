import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider, createBrowserRouter } from 'react-router-dom';
import { QueryClientProvider } from '@tanstack/react-query';
import AppShell from './components/AppShell';
import EmployeePortal from './routes/EmployeePortal';
import ManagerConsole from './routes/ManagerConsole';
import FinanceConsole from './routes/FinanceConsole';
import './styles/global.css';
import queryClient from './api/queryClient';

const router = createBrowserRouter([
  {
    path: '/',
    element: <AppShell />,
    children: [
      { index: true, element: <EmployeePortal /> },
      { path: '/manager', element: <ManagerConsole /> },
      { path: '/finance', element: <FinanceConsole /> }
    ]
  }
]);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </React.StrictMode>
);
