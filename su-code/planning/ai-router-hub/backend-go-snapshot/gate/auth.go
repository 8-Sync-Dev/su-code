package gate

import (
	"context"

	"encore.dev/beta/auth"
	"encore.dev/beta/errs"
)

// AuthData là danh tính đã resolve gắn vào request sau authHandler.
// M2 sẽ mở rộng (quota, grants); M0 chỉ MemberID + Role.
type AuthData struct {
	MemberID string
	Role     string
}

// AuthHandler parse Bearer token. Thiếu/sai -> errs.Unauthenticated (HTTP 401).
// M0 skeleton: 1 token test cứng; resolve per-member key thật = M2 (lookup store).
//
//encore:authhandler
func AuthHandler(ctx context.Context, token string) (auth.UID, *AuthData, error) {
	if token == "" {
		return "", nil, &errs.Error{Code: errs.Unauthenticated, Message: "missing bearer token"}
	}
	if token != "test-token" {
		return "", nil, &errs.Error{Code: errs.Unauthenticated, Message: "invalid token"}
	}
	return auth.UID("test-member"), &AuthData{MemberID: "test-member", Role: "member"}, nil
}
