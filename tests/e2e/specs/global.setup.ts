import { test as setup, expect } from '@playwright/test';

/**
 * Global setup — runs once before all test projects.
 * Ensures the Prometheus server is healthy and Aegis-DB is accessible.
 */
setup('verify server health', async ({ request }) => {
  const health = await request.get('/health');
  expect(health.ok()).toBeTruthy();
  const body = await health.json();
  expect(body.status).toBe('healthy');
});

setup('verify Aegis-DB connectivity', async ({ request }) => {
  // Login to verify Aegis-DB is reachable
  const login = await request.post('/api/v1/auth/login', {
    data: {
      username: process.env.TEST_ADMIN_USER || 'admin',
      password: process.env.TEST_ADMIN_PASS || 'admin_password',
    },
  });
  expect(login.ok()).toBeTruthy();
  const body = await login.json();
  expect(body.token).toBeTruthy();
  expect(body.user).toBeTruthy();
});
