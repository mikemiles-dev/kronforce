## ADDED Requirements

### Requirement: Public endpoint rate limiting
The system SHALL enforce a per-source-IP rate limit on unauthenticated (public) endpoints. The default limit SHALL be 30 requests per minute. When the limit is exceeded, the system SHALL return HTTP 429.

#### Scenario: Request within public limit
- **WHEN** a client IP has made fewer than 30 requests in the last minute to public endpoints
- **THEN** the request proceeds normally

#### Scenario: Request exceeds public limit
- **WHEN** a client IP has made 30 or more requests in the last minute to public endpoints
- **THEN** the system returns HTTP 429 with a JSON body `{"error": "rate limit exceeded, retry after N seconds"}` and a `Retry-After` header

#### Scenario: Public limit window resets
- **WHEN** a client IP exceeded the limit but 60 seconds have elapsed since the first request in the window
- **THEN** subsequent requests are allowed again

### Requirement: Authenticated endpoint rate limiting
The system SHALL enforce a per-API-key rate limit on authenticated user endpoints. The default limit SHALL be 120 requests per minute. The key for rate limiting SHALL be the API key's UUID, extracted from request extensions after auth middleware runs.

#### Scenario: Request within authenticated limit
- **WHEN** an API key has made fewer than 120 requests in the last minute to authenticated endpoints
- **THEN** the request proceeds normally

#### Scenario: Request exceeds authenticated limit
- **WHEN** an API key has made 120 or more requests in the last minute to authenticated endpoints
- **THEN** the system returns HTTP 429 with a JSON body and `Retry-After` header

#### Scenario: Different API keys are independent
- **WHEN** API key A has exceeded its limit
- **THEN** API key B's requests are unaffected

### Requirement: Agent endpoint rate limiting
The system SHALL enforce a per-API-key rate limit on agent-facing endpoints with a higher default limit of 600 requests per minute, since agents legitimately send frequent heartbeats and queue polls.

#### Scenario: Agent request within limit
- **WHEN** an agent API key has made fewer than 600 requests in the last minute to agent endpoints
- **THEN** the request proceeds normally

#### Scenario: Agent request exceeds limit
- **WHEN** an agent API key has made 600 or more requests in the last minute to agent endpoints
- **THEN** the system returns HTTP 429 with a JSON body and `Retry-After` header

### Requirement: Rate limit response headers
All rate-limited responses SHALL include standard rate limit headers to help clients self-regulate.

#### Scenario: 429 response includes headers
- **WHEN** a request is rejected due to rate limiting
- **THEN** the response SHALL include `Retry-After` (seconds until window resets), `X-RateLimit-Limit` (tier's max requests per minute), and `X-RateLimit-Remaining` set to `0`

#### Scenario: Successful response includes remaining count
- **WHEN** a request is allowed through a rate-limited tier
- **THEN** the response SHALL include `X-RateLimit-Limit` and `X-RateLimit-Remaining` headers showing the tier limit and how many requests remain in the current window

### Requirement: Rate limit configuration
Rate limits SHALL be configurable via environment variables. Setting a limit to `0` SHALL disable that tier. A master switch SHALL allow disabling all rate limiting.

#### Scenario: Custom limit via environment variable
- **WHEN** `KRONFORCE_RATE_LIMIT_AUTHENTICATED=60` is set
- **THEN** the authenticated tier enforces 60 requests per minute instead of the default 120

#### Scenario: Disable a specific tier
- **WHEN** `KRONFORCE_RATE_LIMIT_PUBLIC=0` is set
- **THEN** the public tier rate limiting is disabled and all public requests are allowed

#### Scenario: Master disable switch
- **WHEN** `KRONFORCE_RATE_LIMIT_ENABLED=false` is set
- **THEN** no rate limiting is applied to any endpoint

#### Scenario: Default configuration
- **WHEN** no rate limit environment variables are set
- **THEN** rate limiting is enabled with defaults: public=30, authenticated=120, agent=600

### Requirement: Stale entry cleanup
The system SHALL periodically clean up rate limit tracking entries for clients that have not made requests recently, to prevent unbounded memory growth.

#### Scenario: Stale entries pruned
- **WHEN** a client IP or API key has not made any requests in the last 2 minutes
- **THEN** the system removes its tracking entry from the in-memory rate limit state

#### Scenario: Cleanup runs periodically
- **WHEN** the controller is running
- **THEN** stale entry cleanup SHALL run at least once per minute
