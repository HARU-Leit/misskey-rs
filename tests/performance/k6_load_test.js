/**
 * k6 Load Test for misskey-rs API
 *
 * Install k6: https://k6.io/docs/getting-started/installation/
 * Run: k6 run tests/performance/k6_load_test.js
 *
 * Configuration via environment variables:
 *   K6_BASE_URL - API base URL (default: http://localhost:3000)
 *   K6_TOKEN - API token for authenticated requests
 */

import http from 'k6/http';
import { check, sleep, group } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const errorRate = new Rate('errors');
const timelineTrend = new Trend('timeline_duration');
const noteTrend = new Trend('note_duration');

// Configuration
const BASE_URL = __ENV.K6_BASE_URL || 'http://localhost:3000';
const TOKEN = __ENV.K6_TOKEN || '';

// Test options
export const options = {
    stages: [
        // Ramp up
        { duration: '30s', target: 10 },
        { duration: '1m', target: 50 },
        // Sustained load
        { duration: '3m', target: 50 },
        // Peak load
        { duration: '1m', target: 100 },
        { duration: '1m', target: 100 },
        // Ramp down
        { duration: '30s', target: 0 },
    ],
    thresholds: {
        // API response time thresholds from RUST_FORK_PLAN.md
        'http_req_duration': ['p(50)<20', 'p(99)<100'],
        'errors': ['rate<0.01'], // Less than 1% errors
        'timeline_duration': ['p(95)<50'],
        'note_duration': ['p(95)<30'],
    },
};

// Helper function to make API requests
function apiRequest(method, endpoint, body = null) {
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

// Test scenarios
export default function () {
    group('Public Timeline', function () {
        const start = new Date();
        const response = apiRequest('POST', '/api/notes/local-timeline', { limit: 20 });

        const duration = new Date() - start;
        timelineTrend.add(duration);

        const success = check(response, {
            'timeline returns 200': (r) => r.status === 200,
            'timeline returns array': (r) => {
                try {
                    const body = JSON.parse(r.body);
                    return Array.isArray(body);
                } catch {
                    return false;
                }
            },
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    group('Global Timeline', function () {
        const start = new Date();
        const response = apiRequest('POST', '/api/notes/global-timeline', { limit: 20 });

        const duration = new Date() - start;
        timelineTrend.add(duration);

        const success = check(response, {
            'global timeline returns 200': (r) => r.status === 200,
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    group('Note Show', function () {
        // This would need a real note ID in production
        const start = new Date();
        const response = apiRequest('POST', '/api/notes/show', { noteId: 'test-note-id' });

        const duration = new Date() - start;
        noteTrend.add(duration);

        // 404 is acceptable for test note ID
        const success = check(response, {
            'note show returns valid status': (r) => r.status === 200 || r.status === 404,
        });

        errorRate.add(!success);
    });

    sleep(0.1);

    // Only run authenticated tests if token is provided
    if (TOKEN) {
        group('Authenticated: Home Timeline', function () {
            const start = new Date();
            const response = apiRequest('POST', '/api/notes/timeline', { limit: 20 });

            const duration = new Date() - start;
            timelineTrend.add(duration);

            const success = check(response, {
                'home timeline returns 200': (r) => r.status === 200,
            });

            errorRate.add(!success);
        });

        sleep(0.1);

        group('Authenticated: User Show', function () {
            const response = apiRequest('POST', '/api/users/show', { username: 'test' });

            const success = check(response, {
                'user show returns valid status': (r) => r.status === 200 || r.status === 404,
            });

            errorRate.add(!success);
        });
    }

    sleep(0.5);
}

// Setup function - runs once before tests
export function setup() {
    console.log(`Testing API at: ${BASE_URL}`);
    console.log(`Authentication: ${TOKEN ? 'Enabled' : 'Disabled'}`);

    // Verify API is reachable
    const response = http.get(`${BASE_URL}/api/meta`);
    if (response.status !== 200) {
        console.warn(`Warning: API meta endpoint returned ${response.status}`);
    }

    return { baseUrl: BASE_URL };
}

// Teardown function - runs once after tests
export function teardown(data) {
    console.log('Load test completed');
}
