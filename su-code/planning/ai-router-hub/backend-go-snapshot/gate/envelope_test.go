package gate

import "testing"

func TestOk(t *testing.T) {
	r := ok("hi", map[string]any{"a": 1})
	if !r.Success || r.Message != "hi" || r.Result == nil {
		t.Fatalf("ok() sai: %+v", r)
	}
}

func TestFail(t *testing.T) {
	r := fail("boom")
	if r.Success || r.Message != "boom" || r.Result != nil {
		t.Fatalf("fail() sai: %+v", r)
	}
}
