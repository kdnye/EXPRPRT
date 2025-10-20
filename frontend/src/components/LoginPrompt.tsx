import { FormEvent, useMemo, useState } from 'react';
import { request } from '@/api/client';
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
  const [audience, setAudience] = useState<'employee' | 'team'>('employee');

  const audienceCopy = useMemo(
    () =>
      audience === 'employee'
        ? {
            title: 'Employee access',
            description:
              'Sign in with your HR identifier to start a new expense report or pick up a saved draft.',
            credentialLabel: 'Employee credential',
            credentialHint: 'This is the passphrase shared with hourly and salaried staff.'
          }
        : {
            title: 'Manager & finance access',
            description:
              'Approvers and finance specialists can review submitted reports and manage export-ready batches.',
            credentialLabel: 'Team credential',
            credentialHint: 'Need access? Contact Finance Operations to be added to the approval roster.'
          },
    [audience]
  );

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
      setError('We could not verify those details. Double-check your HR identifier and credential.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div className="login-prompt">
      <div className="login-prompt__toggle" role="tablist" aria-label="Select a portal">
        <button
          type="button"
          role="tab"
          aria-selected={audience === 'employee'}
          className={audience === 'employee' ? 'active' : ''}
          onClick={() => setAudience('employee')}
        >
          Employee workspace
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={audience === 'team'}
          className={audience === 'team' ? 'active' : ''}
          onClick={() => setAudience('team')}
        >
          Manager & finance tools
        </button>
      </div>
      <div className="login-prompt__header" role="tabpanel">
        <h2>{audienceCopy.title}</h2>
        <p>{audienceCopy.description}</p>
      </div>
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
        <label htmlFor="credential">{audienceCopy.credentialLabel}</label>
        <input
          id="credential"
          type="password"
          value={credential}
          onChange={(event) => setCredential(event.target.value)}
          autoComplete="current-password"
          required
        />
        <small className="login-prompt__hint">{audienceCopy.credentialHint}</small>
        <button type="submit" disabled={isSubmitting}>
          {isSubmitting ? 'Signing in…' : 'Sign in'}
        </button>
        {error && <p className="login-prompt__error">{error}</p>}
      </form>
      <p className="login-prompt__support">
        Finance exports remain a restricted capability. Employees land on expense entry by default, and approved
        managers can request finance access through the control center once signed in.
      </p>
    </div>
  );
};

export default LoginPrompt;
