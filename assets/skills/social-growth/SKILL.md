---
name: social-growth
description: >-
  OPT-IN (không auto-bật). Use ONLY when the user explicitly enables this skill or asks to build social-media marketing / branding / lead-gen for 8 Sync Dev — campaigns on Facebook, YouTube, TikTok (and Fanpage / community groups), setting up a Page properly, audience insight, monthly content plan + targets/KPIs, or finding leads & customers. Composes with `assp-skill` (brand voice + avatar + offer) and `last30days` (recency research); never replaces them. Produces a concrete monthly plan with per-channel targets, content calendar, funnel, and lead-capture mechanics — never generic "post consistently" advice.
---

# social-growth — chiến dịch truyền thông + tìm khách cho 8 Sync Dev

> **OPT-IN.** Skill này KHÔNG auto-bật. Chỉ dùng khi user bật (`8sync skill add builtin:social-growth`)
> hoặc yêu cầu rõ về social/branding/leads. Khi task chạm tới copy/offer/giá → đọc `assp-skill`. Khi
> cần "thị trường đang nói gì" → dùng `last30days`. Skill này lo PHẦN PHÂN PHỐI + FUNNEL + ĐO LƯỜNG.

## 0. Nguyên tắc (đọc trước)

1. **Một avatar, một offer, một kênh chủ lực** (theo `assp-skill`). Đừng dàn trải 3 kênh ngang nhau ngày 1.
2. **Plan phải có SỐ.** Mỗi đề xuất = mục tiêu đo được (reach / leads / CAC), không "tăng tương tác".
3. **Hook 3 giây** quyết định mọi nền tảng. Nội dung không có hook = không phân phối.
4. **Organic trước, paid sau.** Chỉ scale paid khi 1 content organic đã chứng minh giữ chân (retention/CTR).
5. **Mọi kênh dẫn về 1 nơi bắt lead** (group / form / DM / landing). View không có đường về = lãng phí.

## 1. Chọn kênh theo mục tiêu (đừng làm hết cùng lúc)

| Kênh | Mạnh nhất cho | Format thắng | Nhịp tối thiểu |
|---|---|---|---|
| **TikTok** | Reach lạnh nhanh, test hook rẻ | Video dọc 15–45s, hook 2s, 1 ý/clip | 1/ngày (test) |
| **YouTube** | Tin tưởng sâu, search lâu dài, demo kỹ thuật | Long-form 8–15p + Shorts cắt từ long | 1 long/tuần + 3 shorts |
| **Facebook Page** | Remarketing, cộng đồng, chạy ads | Post 200–400 chữ + carousel, video | 3–5/tuần |
| **Facebook Group / Cộng đồng** | Nuôi lead, bán mềm, social proof | Thảo luận, case, Q&A, mini-challenge | tương tác hằng ngày |

**Quy tắc chọn:** sản phẩm cần *demo/độ tin* (8 Sync Coder, Vector DB) → YouTube chủ lực. Sản phẩm cần
*reach trẻ + cảm xúc* (IELTS AI) → TikTok chủ lực. B2B/SME (CRM) → Facebook + cộng đồng + ads remarketing.

## 2. Thiết lập Page/kênh ĐÚNG (checklist 1 lần)

- **Định vị 1 dòng** trên bio: "[Sản phẩm] giúp [avatar] đạt [dream outcome] bằng [mechanism]" (lấy từ `assp` Step 1/3).
- **Avatar/cover** nhất quán brand 8 Sync Dev; footer/nhận diện theo `assp` mục 1.
- **CTA cố định**: nút Liên hệ / link landing / pinned post dẫn về nơi bắt lead.
- **Pinned/Featured**: 1 post "bắt đầu từ đây" (lead magnet) + 1 social proof.
- **Link in bio** = 1 đích duy nhất (không list 10 link). Ưu tiên: lead magnet free → sản phẩm trả phí.
- **Tracking**: UTM cho mọi link; Pixel/Tag (FB Pixel, GA4) trước khi chạy bất kỳ ads nào.

## 3. Insight & Audience (làm TRƯỚC khi sản xuất)

1. **Avatar** → lấy từ `assp-skill` Step 1 (tên, pain 3h sáng, dream outcome, internal language verbatim).
2. **Recency** → `last30days`: 5–10 hook/đau thực tế audience đang nói trong 30 ngày (Reddit/TikTok/YT comment).
3. **Đối thủ** → 3 kênh đối thủ: lọc 5 video top theo view/engagement → rút *format + hook + offer* họ dùng.
4. **Insight output** (1 trang): top 5 pain (verbatim) · 10 hook ăn theo internal language · 3 format đang viral
   trong ngách · 1 "góc nhìn duy nhất" của mình (Hero Mechanism từ `assp` Step 3).

## 4. Funnel (mọi content phải biết nó ở tầng nào)

```
TOFU (lạ → biết)     : TikTok/Shorts/Reels — hook đau, 1 ý, CTA mềm "follow để xem tiếp"
MOFU (biết → tin)    : YouTube long-form, case study, demo, so sánh — CTA "tải lead magnet"
BOFU (tin → mua)     : webinar/live, testimonial, offer có deadline — CTA "đăng ký/đặt chỗ"
Lead capture         : group + form/landing + DM auto-reply (keyword) → email/Zalo
Nurture → Close      : chuỗi email/Zalo theo offer (assp Step 4/5), waived-fee close
```

**Tỉ trọng nội dung gợi ý:** 60% TOFU (reach) · 25% MOFU (tin) · 15% BOFU (chốt). Lệch về TOFU lúc mới, dồn
BOFU quanh đợt mở bán/cohort.

## 5. Plan THÁNG + Target (deliverable chính)

Khi user hỏi "lên plan tháng", sản xuất file `social-plan-{thang}-{sản_phẩm}.md` gồm ĐỦ:

1. **North-star tháng**: 1 con số (vd: 200 lead vào group / 30 booking demo / 1.000 email).
2. **Funnel target ngược từ doanh thu**:
   `Doanh thu mục tiêu → #khách → #lead (theo close rate) → #reach (theo CTR→lead)`. Ghi rõ giả định %.
3. **Mục tiêu theo kênh** (bảng): kênh · output (số post/video) · KPI chính (reach/CTR/lead) · KPI phụ.
4. **Content calendar**: 4 tuần × nhịp kênh (mục 1), mỗi slot = {format, hook, funnel-stage, CTA}. Ít nhất
   tuần 1 chi tiết từng ngày; tuần 2–4 theo theme.
5. **3–5 theme tháng** (xoay quanh 1 Hero Mechanism, không tản mạn).
6. **Budget** (nếu có ads): chia test vs scale; rule "chỉ scale content đã pass organic".
7. **Review cadence**: chốt ngày review tuần + ngưỡng quyết định (mục 7).

**Ví dụ target (8 Sync Coder, tháng mở cohort):**
- North-star: 100 đăng ký VFLOW Challenge.
- Ngược: 100 đăng ký ← 400 lead (close 25%) ← 40.000 reach đủ điều kiện (lead-rate 1%).
- Kênh: TikTok 30 clips (reach 30k) · YouTube 4 long + 8 shorts (watch-time + 1k lead) · FB group nuôi 400 lead.

## 6. Tìm Lead / Khách (organic + paid)

- **Organic**: trả lời thật trong cộng đồng ngách (không spam link) · mini-challenge 3–7 ngày bắt email ·
  comment-to-DM (keyword auto-reply) · "carousel giá trị" dẫn về lead magnet.
- **Lead magnet** đúng avatar: checklist/template/mini-tool free (xem `assp` — free product = Attraction Offer).
- **Paid** (sau khi organic chứng minh): TikTok Spark Ads boost đúng clip đã viral · FB lookalike từ pixel
  lead · YouTube ads remarketing người xem ≥50%. Mục tiêu paid = **giảm CAC**, không "mua view".
- **DM funnel**: hook → qualify 1 câu → đưa lead magnet → mời vào group/booking. Không pitch ở tin đầu.
- **CFA check** (assp Step 4): `30-day GP ≥ 2×(CAC+COGS)?` Không đạt → sửa offer/hook trước khi tăng budget.

## 7. Đo lường & vòng lặp insight (quyết định bằng số)

| Tầng | Chỉ số "đèn" | Ngưỡng hành động |
|---|---|---|
| Hook (TOFU) | 3s view rate / retention 25% | < ngưỡng → đổi hook, KHÔNG đổi cả nội dung |
| Phân phối | CTR ra link / save / share | thấp → CTA yếu hoặc sai funnel-stage |
| Lead | lead-rate, cost/lead | tăng đều → scale; tăng vọt cost → tạm dừng ads |
| Close | book→mua, reply rate | thấp → offer/nurture (về `assp` Step 5) |

- **Weekly review**: 1 bảng, top/flop 3 content, 1 quyết định "double-down / kill / iterate".
- **Quy tắc:** kill content < ngưỡng sau 48h; double-down clip pass retention bằng cách cắt biến thể + boost.
- **Insight ghi lại** vào `su-code/KNOWLEDGE.md` để tháng sau không lặp lỗi.

## 8. Bàn giao

- Output mặc định: `social-plan-{thang}-{sản_phẩm}.md` (mục 5) + (nếu xin) 1 batch hook/script theo `assp` voice.
- Mọi copy/script **bắt buộc** qua Voice Profile Alex Dev (`assp-skill` Step 2) — không sến, không buzzword.
- Footer/nhận diện theo `assp` mục 1.

---
*social-growth v1 — opt-in · validate (assp) trước, phân phối (skill này) sau · luôn có SỐ.*
