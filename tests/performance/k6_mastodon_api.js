/**
 * k6 Load Test for Mastodon-compatible API
 *
 * Tests the Mastodon-compatible endpoints for third-party client compatibility.
 *
 * Install k6: https://k6.io/docs/getting-started/installation/
 * Run: k6 run tests/performance/k6_mastodon_api.js
 *
 * Configuration via environment variables:
 *   K6_BASE_URL - API base URL (default: http://localhost:3000)
 *   K6_TOKEN - OAuth token for authenticated requests
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const timelineTrend = new Trend('mastodon_timeline_duration');

// Configuration
const BASE_URL = __ENV.K6_BASE_URL || 'http://localhost:3000';
const TOKEN = __ENV.K6_TOKEN || '';

// Test options - lighter load for Mastodon API
export const options = {
    stages: [
        { duration: '20s', target: 10 },
        { duration: '1m', target: 30 },
        { duration: '2m', target: 30 },
        { duration: '20s', target: 0 },
    ],
    thresholds: {
        'http_req_duration': ['p(50)<30', 'p(99)<150'],
        'errors': ['rate<0.02'],
        'mastodon_timeline_duration': ['p(95)<100'],
    },
};

function mastodonRequest(method, endpoint, body = null) {
    const url = `${BASE_URL}${endpoint}`;
    const params = {
        headers: {
            'Content-Type': 'application/json',
        },
    };

    if (TOKEN) {
        params.headers['Authorization'] = `Bearer ${TOKEN}`;
    }

    let response;
    if (method === 'GET') {
        response = http.get(url, params);
    } else {
        response = http.post(url, body ? JSON.stringify(body) : null, params);
    }

    return response;
}

export default function () {
    // Public endpoints (no auth required)
    group('Mastodon: Public Timeline', function () {
        const start = new Date();
        const response = mastodonRequest('GET', '/api/v1/timelines/public?limit=20');

        const duration = new Date() - start;
        timelineTrend.add(duration);

        const success = check(response, {
            'public timeline returns 200': (r) => r.status === 200,
            'returns array': (r) => {
                try {
                    return Array.isArray(JSON.parse(r.body));
                } catch {
                    return false;
                }
            },
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    group('Mastodon: Local Timeline', function () {
        const start = new Date();
        const response = mastodonRequest('GET', '/api/v1/timelines/public?local=true&limit=20');

        const duration = new Date() - start;
        timelineTrend.add(duration);

        const success = check(response, {
            'local timeline returns 200': (r) => r.status === 200,
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    // Authenticated endpoints
    if (TOKEN) {
        group('Mastodon: Home Timeline', function () {
            const start = new Date();
            const response = mastodonRequest('GET', '/api/v1/timelines/home?limit=20');

            const duration = new Date() - start;
            timelineTrend.add(duration);

            const success = check(response, {
                'home timeline returns 200': (r) => r.status === 200,
            });

            errorRate.add(!success);
        });

        sleep(0.1);

        group('Mastodon: Verify Credentials', function () {
            const response = mastodonRequest('GET', '/api/v1/accounts/verify_credentials');

            const success = check(response, {
                'verify credentials returns 200': (r) => r.status === 200,
                'returns account object': (r) => {
                    try {
                        const body = JSON.parse(r.body);
                        return body.id && body.username;
                    } catch {
                        return false;
                    }
                },
            });

            errorRate.add(!success);
        });

        sleep(0.1);

        group('Mastodon: Post Status', function () {
            const response = mastodonRequest('POST', '/api/v1/statuses', {
                status: `Load test status at ${Date.now()}`,
                visibility: 'direct', // Use direct to avoid polluting timelines
            });

            const success = check(response, {
                'post status returns 200': (r) => r.status === 200,
            });

            errorRate.add(!success);
        });
    }

    sleep(0.5);
}

export function setup() {
    console.log(`Testing Mastodon API at: ${BASE_URL}`);
    console.log(`Authentication: ${TOKEN ? 'Enabled' : 'Disabled'}`);

    // Check if Mastodon API is available
    const response = http.get(`${BASE_URL}/api/v1/timelines/public?limit=1`);
    if (response.status !== 200) {
        console.warn(`Warning: Mastodon API may not be available (status: ${response.status})`);
    }

    return { baseUrl: BASE_URL };
}

export function teardown(data) {
    console.log('Mastodon API load test completed');
}
