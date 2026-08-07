package gate

import "context"

// Health là liveness probe công khai (không cần auth) — dùng cho load balancer / uptime.
//
//encore:api public method=GET path=/health
func Health(ctx context.Context) (*Response, error) {
	return ok("ok", map[string]any{
		"service": "ai-router-hub",
		"plane":   "control",
		"phase":   "M0",
	}), nil
}
