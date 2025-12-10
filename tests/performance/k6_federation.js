/**
 * k6 Load Test for ActivityPub Federation Endpoints
 *
 * Tests federation endpoints that receive activities from other instances.
 *
 * Install k6: https://k6.io/docs/getting-started/installation/
 * Run: k6 run tests/performance/k6_federation.js
 *
 * Configuration via environment variables:
 *   K6_BASE_URL - API base URL (default: http://localhost:3000)
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('federation_errors');
const webfingerTrend = new Trend('webfinger_duration');
const actorTrend = new Trend('actor_fetch_duration');
const inboxTrend = new Trend('inbox_duration');

// Configuration
const BASE_URL = __ENV.K6_BASE_URL || 'http://localhost:3000';

// Test options - simulate federated server behavior
export const options = {
    stages: [
        { duration: '15s', target: 5 },
        { duration: '1m', target: 20 },
        { duration: '2m', target: 20 },
        { duration: '15s', target: 0 },
    ],
    thresholds: {
        'webfinger_duration': ['p(95)<50'],
        'actor_fetch_duration': ['p(95)<100'],
        'federation_errors': ['rate<0.05'],
    },
};

export default function () {
    // WebFinger lookup (commonly called by federated servers)
    group('WebFinger Lookup', function () {
        const start = new Date();
        const response = http.get(
            `${BASE_URL}/.well-known/webfinger?resource=acct:admin@${BASE_URL.replace(/https?:\/\//, '')}`,
            {
                headers: {
                    'Accept': 'application/jrd+json',
                },
            }
        );

        const duration = new Date() - start;
        webfingerTrend.add(duration);

        const success = check(response, {
            'webfinger returns 200 or 404': (r) => r.status === 200 || r.status === 404,
            'webfinger has correct content-type': (r) =>
                r.status !== 200 || r.headers['Content-Type']?.includes('application/jrd+json'),
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    // NodeInfo lookup
    group('NodeInfo', function () {
        const response = http.get(`${BASE_URL}/.well-known/nodeinfo`, {
            headers: {
                'Accept': 'application/json',
            },
        });

        const success = check(response, {
            'nodeinfo returns 200': (r) => r.status === 200,
            'nodeinfo has links': (r) => {
                try {
                    const body = JSON.parse(r.body);
                    return body.links && Array.isArray(body.links);
                } catch {
                    return false;
                }
            },
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    // Actor fetch (called when fetching remote user profiles)
    group('Actor Fetch', function () {
        const start = new Date();
        // This would need a real user ID in production
        const response = http.get(`${BASE_URL}/users/admin`, {
            headers: {
                'Accept': 'application/activity+json',
            },
        });

        const duration = new Date() - start;
        actorTrend.add(duration);

        const success = check(response, {
            'actor fetch returns valid status': (r) => r.status === 200 || r.status === 404,
            'actor has correct content-type': (r) =>
                r.status !== 200 || r.headers['Content-Type']?.includes('activity+json'),
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    // Note fetch
    group('Note Fetch', function () {
        const response = http.get(`${BASE_URL}/notes/test-note`, {
            headers: {
                'Accept': 'application/activity+json',
            },
        });

        const success = check(response, {
            'note fetch returns valid status': (r) => r.status === 200 || r.status === 404,
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    // Shared inbox POST (simulated - would need valid signatures in production)
    group('Shared Inbox', function () {
        const activity = {
            '@context': 'https://www.w3.org/ns/activitystreams',
            type: 'Create',
            id: `https://remote.example/activities/${Date.now()}`,
            actor: 'https://remote.example/users/test',
            object: {
                type: 'Note',
                id: `https://remote.example/notes/${Date.now()}`,
                content: 'Test note from federated server',
                attributedTo: 'https://remote.example/users/test',
                to: ['https://www.w3.org/ns/activitystreams#Public'],
            },
        };

        const start = new Date();
        const response = http.post(`${BASE_URL}/inbox`, JSON.stringify(activity), {
            headers: {
                'Content-Type': 'application/activity+json',
                // In production, this would need HTTP Signatures
            },
        });

        const duration = new Date() - start;
        inboxTrend.add(duration);

        // 401/403 is expected without valid signature
        const success = check(response, {
            'inbox accepts or rejects properly': (r) =>
                r.status === 202 || r.status === 401 || r.status === 403 || r.status === 400,
        });

        errorRate.add(!success);
    });

    sleep(0.5);
}

export function setup() {
    console.log(`Testing Federation at: ${BASE_URL}`);

    // Check if federation endpoints are available
    const response = http.get(`${BASE_URL}/.well-known/nodeinfo`);
    if (response.status !== 200) {
        console.warn(`Warning: NodeInfo not available (status: ${response.status})`);
    }

    return { baseUrl: BASE_URL };
}

export function teardown(data) {
    console.log('Federation load test completed');
}
