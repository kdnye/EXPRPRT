import { FormEvent, useState } from 'react';
import { request } from '../api/client';
import { Role, useAuth } from '../hooks/useAuth';
import './LoginPrompt.css';

interface LoginResponse {
  token: string;
  role: Role;
}

const LoginPrompt = () => {
  const { login } = useAuth();
  const [hrIdentifier, setHrIdentifier] = useState('');
  const [credential, setCredential] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setIsSubmitting(true);

    try {
      const response = await request<LoginResponse>('post', '/auth/login', {
        hr_identifier: hrIdentifier,
        credential
      });
      login(response.token, response.role);
    } catch (err) {
      console.error('Failed to log in', err);
      setError('Invalid credential. Please confirm the HR identifier and developer passphrase.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="login-prompt">
      <h2>Developer Login</h2>
      <p>Use your HR identifier and the shared developer credential to access the portals.</p>
      <form onSubmit={handleSubmit}>
        <label htmlFor="hr-identifier">HR Identifier</label>
        <input
          id="hr-identifier"
          type="text"
          value={hrIdentifier}
          onChange={(event) => setHrIdentifier(event.target.value)}
          autoComplete="username"
          required
        />
        <label htmlFor="credential">Developer Credential</label>
        <input
          id="credential"
          type="password"
          value={credential}
          onChange={(event) => setCredential(event.target.value)}
          autoComplete="current-password"
          required
        />
        <button type="submit" disabled={isSubmitting}>
          {isSubmitting ? 'Signing in…' : 'Sign in'}
        </button>
        {error && <p className="login-prompt__error">{error}</p>}
      </form>
    </div>
  );
};

export default LoginPrompt;
