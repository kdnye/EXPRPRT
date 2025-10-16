import { useState } from 'react';

type Role = 'employee' | 'manager' | 'finance';

interface AuthState {
  token?: string;
  role: Role;
}

export const useAuth = () => {
  const [state, setState] = useState<AuthState>({ role: 'employee' });

  const login = (token: string, role: Role) => {
    setState({ token, role });
    localStorage.setItem('fsi-auth-token', token);
  };

  const logout = () => {
    setState({ role: 'employee' });
    localStorage.removeItem('fsi-auth-token');
  };

  return {
    ...state,
    login,
    logout
  };
};
