import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState
} from 'react';

interface AuthBypassConfig {
  enabled: boolean;
  role: Role;
}

export type Role = 'employee' | 'manager' | 'finance' | 'admin';

interface AuthState {
  token?: string;
  role: Role;
}

interface AuthContextValue extends AuthState {
  isAuthenticated: boolean;
  isReady: boolean;
  login: (token: string, role: Role) => void;
  logout: () => void;
}

export const AUTH_TOKEN_STORAGE_KEY = 'fsi-auth-token';
export const AUTH_LOGOUT_EVENT = 'fsi-auth-logout';

const AuthContext = createContext<AuthContextValue | undefined>(undefined);

export const AuthProvider = ({ children }: { children: ReactNode }) => {
  const [token, setToken] = useState<string | undefined>();
  const [role, setRole] = useState<Role>('employee');
  const [isReady, setIsReady] = useState(false);
  const [authBypass] = useState<AuthBypassConfig | undefined>(() => {
    const runtime = window.__FSI_EXPENSES_CONFIG__?.authBypass;
    if (runtime?.enabled) {
      const runtimeRole = coerceRole(runtime.role);
      if (runtimeRole) {
        return { enabled: true, role: runtimeRole };
      }
    }

    const envEnabled = String(import.meta.env.VITE_AUTH_BYPASS ?? '').toLowerCase();
    if (envEnabled === 'true') {
      const envRole = coerceRole(import.meta.env.VITE_AUTH_BYPASS_ROLE);
      return { enabled: true, role: envRole ?? 'employee' };
    }

    return undefined;
  });

  useEffect(() => {
    if (authBypass?.enabled) {
      setRole(authBypass.role);
      setIsReady(true);
      return;
    }

    const storedToken = localStorage.getItem(AUTH_TOKEN_STORAGE_KEY);
    if (storedToken) {
      setToken(storedToken);
    }
    setIsReady(true);

    const handleStorage = (event: StorageEvent) => {
      if (event.key === AUTH_TOKEN_STORAGE_KEY) {
        if (event.newValue) {
          setToken(event.newValue);
        } else {
          setToken(undefined);
          setRole('employee');
        }
      }
    };

    const handleLogoutEvent = () => {
      setToken(undefined);
      setRole('employee');
    };

    window.addEventListener('storage', handleStorage);
    window.addEventListener(AUTH_LOGOUT_EVENT, handleLogoutEvent);

    return () => {
      window.removeEventListener('storage', handleStorage);
      window.removeEventListener(AUTH_LOGOUT_EVENT, handleLogoutEvent);
    };
  }, [authBypass]);

  const login = useCallback((nextToken: string, nextRole: Role) => {
    if (authBypass?.enabled) {
      setRole(authBypass.role);
      return;
    }
    localStorage.setItem(AUTH_TOKEN_STORAGE_KEY, nextToken);
    setToken(nextToken);
    setRole(nextRole);
  }, [authBypass]);

  const logout = useCallback(() => {
    if (authBypass?.enabled) {
      setRole(authBypass.role);
      return;
    }
    localStorage.removeItem(AUTH_TOKEN_STORAGE_KEY);
    setToken(undefined);
    setRole('employee');
    window.dispatchEvent(new Event(AUTH_LOGOUT_EVENT));
  }, [authBypass]);

  const value = useMemo<AuthContextValue>(
    () => ({
      token,
      role,
      isAuthenticated: authBypass?.enabled ? true : Boolean(token),
      isReady,
      login,
      logout
    }),
    [token, role, isReady, login, logout, authBypass]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
};

export const useAuth = () => {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
};

const coerceRole = (value: unknown): Role | undefined => {
  if (value === 'employee' || value === 'manager' || value === 'finance' || value === 'admin') {
    return value;
  }
  return undefined;
};
