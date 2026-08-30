# STATE (8sync managed — live plan; rewrite ở MỖI phase-boundary, đọc đầu phiên)
> **Active feature:** Reconcile su-code-v058 & Fix GLM-5.3-Flash native vision across all projects.

## Goal
Hợp nhất toàn bộ commit và working changes từ `C:\Users\Projects\su-code-v058` vào repo chuẩn `C:\Users\Projects\su-code`, xóa bỏ thư mục thừa `su-code-v058`, khắc phục triệt để luồng đọc ảnh của GLM-5.3-Flash (phân định rõ model hỗ trợ vision trong catalog omp vs tool `read`), build release và sweep cập nhật toàn bộ 13+ dự án trên máy.

## Checklist
- [ ] Slice 1: Reconcile workspace — di dời .git + changes từ `su-code-v058` sang `su-code`, verify git log/status rồi xóa an toàn `su-code-v058`
- [ ] Slice 2: Calibrate GLM-5.3-Flash vision routing — alias trỏ chuẩn `zai/glm-5.3-flash`, cập nhật `APPEND_SYSTEM.md` + `image-routing` hướng dẫn agent dùng tool `read` để nạp ảnh
- [ ] Slice 3: Build release binary, atomic install vào `%LOCALAPPDATA%\Programs\8sync\8sync.exe`, sweep `--force` toàn bộ dự án `C:\Users\Projects` và smoke-test vision

## Current
Plan đã được lập chi tiết qua `engine_plan` (3 slices, 7 tasks) và đã qua review độc lập của `PlanReviewer`. Sẵn sàng thực thi.

## Next
Chạy `/sx-auto` để thực thi tuần tự 3 slices theo engine plan.

## Assumptions & Evidence
1. `C:\Users\Projects\su-code-v058` chứa git history thật trên nhánh `fix/glm-53-native-vision` (commit `8e95f1d`) cùng 18 file modified. `C:\Users\Projects\su-code` là clone rỗng chưa commit.
2. `omp models --json` định nghĩa `zai/glm-5.3-flash` có `"input": ["text", "image"]`, trong khi `zai/glm-5.3` chỉ có `"input": ["text"]`.
3. Tool `read` trong harness omp tự động decode file ảnh thành khối `[image/webp]` chuyển tới LLM nếu model hỗ trợ vision.
4. Lệnh sweep cần cờ `--force` (`8sync harness create --force --sweep`) để đè các skill và system prompt cũ trên 13+ dự án hiện hữu.

## Contracts & Interfaces
- Binary target: `%LOCALAPPDATA%\Programs\8sync\8sync.exe`
- Model default vision alias: `glm`, `zai`, `flash` → `zai/glm-5.3-flash`
- Project sweep target: `C:\Users\Projects`

## Non-goals
- Không can thiệp vào các model text-only ngoài việc giữ fallback `zai-vision` cho GLM-5.2 trở xuống.
- Không xóa bỏ bất kỳ file code nghiệp vụ của các dự án con khi thực hiện sweep.
