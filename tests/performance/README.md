# Performance Tests

This directory contains load tests for misskey-rs using k6.

## Prerequisites

Install k6: https://k6.io/docs/getting-started/installation/

```bash
# macOS
brew install k6

# Linux (Debian/Ubuntu)
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# Windows (Chocolatey)
choco install k6

# Docker
docker pull grafana/k6
```

## Test Files

- `k6_load_test.js` - Main API load test (Misskey API)
- `k6_mastodon_api.js` - Mastodon-compatible API load test
- `k6_federation.js` - ActivityPub federation endpoints test

## Running Tests

### Basic API Test

```bash
# Default (localhost:3000)
k6 run tests/performance/k6_load_test.js

# With custom URL
K6_BASE_URL=http://localhost:8080 k6 run tests/performance/k6_load_test.js

# With authentication
K6_BASE_URL=http://localhost:3000 K6_TOKEN=your-token-here k6 run tests/performance/k6_load_test.js
```

### Mastodon API Test

```bash
k6 run tests/performance/k6_mastodon_api.js

# With OAuth token
K6_TOKEN=your-oauth-token k6 run tests/performance/k6_mastodon_api.js
```

### Federation Test

```bash
k6 run tests/performance/k6_federation.js
```

## Running with Docker

```bash
docker run --rm -i grafana/k6 run - <tests/performance/k6_load_test.js

# With network access to host
docker run --rm -i --network host grafana/k6 run - <tests/performance/k6_load_test.js
```

## Performance Targets

From RUST_FORK_PLAN.md:

| Metric | Target |
|--------|--------|
| API response time (p50) | < 20ms |
| API response time (p99) | < 100ms |
| Memory usage | < 256MB (idle) |
| Startup time | < 2s |

## Output Options

### HTML Report

```bash
k6 run --out web-dashboard tests/performance/k6_load_test.js
```

### JSON Output

```bash
k6 run --out json=results.json tests/performance/k6_load_test.js
```

### InfluxDB (for Grafana)

```bash
k6 run --out influxdb=http://localhost:8086/k6 tests/performance/k6_load_test.js
```

## Custom Test Scenarios

You can modify the `options.stages` in each test file to customize the load pattern:

```javascript
export const options = {
    stages: [
        // Ramp-up
        { duration: '1m', target: 100 },
        // Sustained load
        { duration: '5m', target: 100 },
        // Spike
        { duration: '30s', target: 500 },
        { duration: '1m', target: 500 },
        // Recovery
        { duration: '2m', target: 100 },
        // Ramp-down
        { duration: '30s', target: 0 },
    ],
};
```

## Interpreting Results

Key metrics to watch:

- `http_req_duration` - Request latency (p50, p95, p99)
- `http_reqs` - Total requests per second
- `errors` - Error rate (should be < 1%)
- `vus` - Virtual users active

Example output:

```
     ✓ timeline returns 200
     ✓ timeline returns array

     checks.........................: 100.00% ✓ 5000  ✗ 0
     data_received..................: 25 MB   417 kB/s
     data_sent......................: 1.2 MB  20 kB/s
     http_req_duration..............: avg=15ms p(50)=12ms p(99)=45ms
     http_reqs......................: 5000    83.33/s
     iteration_duration.............: avg=1.2s
     vus............................: 50      min=0   max=100
```

## Troubleshooting

### Connection refused

Make sure the server is running and accessible:

```bash
curl http://localhost:3000/api/meta
```

### High error rate

Check server logs for errors. Common issues:
- Database connection limits
- Redis connection issues
- Memory exhaustion

### Slow response times

Profile the server:
- Check database query times
- Monitor Redis operations
- Look for N+1 queries
