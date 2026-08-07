package gate

import (
	"context"

	"encore.dev/beta/auth"
)

// WhoAmI trả danh tính đã auth — endpoint tối thiểu để chứng minh authHandler wiring.
//
//encore:api auth method=GET path=/whoami
func WhoAmI(ctx context.Context) (*Response, error) {
	uid, _ := auth.UserID()
	role := ""
	if data, okCast := auth.Data().(*AuthData); okCast && data != nil {
		role = data.Role
	}
	return ok("whoami", map[string]any{
		"uid":  string(uid),
		"role": role,
	}), nil
}
