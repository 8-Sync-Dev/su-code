package gate

// Response là envelope thống nhất cho mọi endpoint của control plane.
// Khớp convention mind0/zus: {success, message, result}.
type Response struct {
	Success bool        `json:"success"`
	Message string      `json:"message"`
	Result  interface{} `json:"result,omitempty"`
}

// ok dựng envelope thành công.
func ok(msg string, result interface{}) *Response {
	return &Response{Success: true, Message: msg, Result: result}
}

// fail dựng envelope lỗi (dùng cho các nhánh trả 200 nhưng success=false;
// lỗi HTTP thật sự đi qua encore.dev/beta/errs).
func fail(msg string) *Response {
	return &Response{Success: false, Message: msg}
}
