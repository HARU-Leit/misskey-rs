# Local Federation Tests

This directory contains infrastructure for testing ActivityPub federation between two local instances.

## Overview

The federation tests verify that two misskey-rs instances can properly communicate with each other using ActivityPub protocol.

## Prerequisites

- Docker and Docker Compose
- Two PostgreSQL databases (provided by docker-compose.test.yml)
- Two Redis instances (provided by docker-compose.test.yml)

## Setup

### 1. Start the test infrastructure

```bash
# Start databases and Redis for both instances
docker-compose -f docker-compose.test.yml --profile federation up -d
```

### 2. Environment variables

Instance 1 (default):
```bash
export MISSKEY_URL=http://localhost:3001
export DATABASE_URL=postgres://misskey_test:misskey_test@localhost:5433/misskey_test
export REDIS_URL=redis://localhost:6380
export PORT=3001
```

Instance 2:
```bash
export MISSKEY_URL=http://localhost:3002
export DATABASE_URL=postgres://misskey_test:misskey_test@localhost:5434/misskey_test_2
export REDIS_URL=redis://localhost:6381
export PORT=3002
```

### 3. Start both instances

Terminal 1:
```bash
PORT=3001 DATABASE_URL=postgres://misskey_test:misskey_test@localhost:5433/misskey_test \
  REDIS_URL=redis://localhost:6380 MISSKEY_URL=http://localhost:3001 \
  cargo run --release
```

Terminal 2:
```bash
PORT=3002 DATABASE_URL=postgres://misskey_test:misskey_test@localhost:5434/misskey_test_2 \
  REDIS_URL=redis://localhost:6381 MISSKEY_URL=http://localhost:3002 \
  cargo run --release
```

## Test Scenarios

### 1. WebFinger Discovery

Verify that each instance can discover users on the other:

```bash
# From instance 1, discover user on instance 2
curl "http://localhost:3001/.well-known/webfinger?resource=acct:testuser@localhost:3002"

# From instance 2, discover user on instance 1
curl "http://localhost:3002/.well-known/webfinger?resource=acct:testuser@localhost:3001"
```

### 2. Actor Fetch

Verify that instances can fetch actor information:

```bash
# Fetch actor from instance 1
curl -H "Accept: application/activity+json" http://localhost:3001/users/testuser

# Fetch actor from instance 2
curl -H "Accept: application/activity+json" http://localhost:3002/users/testuser
```

### 3. Follow Activity

Test follow workflow between instances:

1. User on instance 1 follows user on instance 2
2. Verify Follow activity is delivered to instance 2's inbox
3. Verify Accept activity is sent back to instance 1
4. Verify follower/following counts are updated

### 4. Note Federation

Test note creation and delivery:

1. Create a public note on instance 1
2. Verify note appears in instance 2's federated timeline
3. Test mentions to users on the other instance

### 5. Reaction Federation

Test reaction synchronization:

1. Create a note on instance 1
2. Add reaction from user on instance 2
3. Verify reaction count is updated on instance 1

## Automated Tests

Run the automated federation tests:

```bash
# Start test infrastructure first
docker-compose -f docker-compose.test.yml --profile federation up -d

# Run federation tests (requires both instances running)
cargo test --test federation_e2e -- --ignored
```

## Troubleshooting

### Connection refused

- Ensure both instances are running
- Check that ports 3001 and 3002 are not in use

### Signature verification failed

- Check that system clocks are synchronized
- Verify RSA keys are properly generated for test users

### Activity not delivered

- Check the delivery queue logs
- Verify the target inbox URL is correct
- Check for HTTP errors in the delivery worker

## Cleanup

```bash
# Stop and remove test containers
docker-compose -f docker-compose.test.yml --profile federation down -v
```
