package gate

import (
	"context"
	"testing"

	"encore.dev/beta/errs"
)

// unauthorized soi field Code trực tiếp (không dùng errs.Code() — helper đó cần
// Encore runtime nên panic dưới `go test` thuần; `encore test` mới có runtime).
func unauthorized(err error) bool {
	e, ok := err.(*errs.Error)
	return ok && e.Code == errs.Unauthenticated
}

func TestAuthHandler(t *testing.T) {
	ctx := context.Background()

	if _, _, err := AuthHandler(ctx, ""); !unauthorized(err) {
		t.Fatalf("empty token: mong Unauthenticated, được %v", err)
	}
	if _, _, err := AuthHandler(ctx, "nope"); !unauthorized(err) {
		t.Fatalf("bad token: mong Unauthenticated, được %v", err)
	}
	uid, data, err := AuthHandler(ctx, "test-token")
	if err != nil || uid == "" || data == nil || data.Role != "member" {
		t.Fatalf("good token: uid=%q data=%+v err=%v", uid, data, err)
	}
}
