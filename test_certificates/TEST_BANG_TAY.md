# 🧪 CÁC BƯỚC TEST AUTO-SIGN BẰNG TAY

## 📋 CHUẨN BỊ

### 1. Kiểm tra server đang chạy
```bash
ps aux | grep letmesign
```

Nếu chưa chạy:
```bash
cd /home/giap/giap/Docuseal_Pro
cargo run > /tmp/docuseal.log 2>&1 &
```

### 2. Kiểm tra certificate đã setup
```bash
PGPASSWORD=letmesignpassword psql -h 192.168.90.11 -U letmesign -d letmesigndb \
  -c "SELECT id, name, is_default, enable_auto_sign FROM certificates WHERE id = 68;"
```

**Phải thấy:**
```
 id | name | is_default | enable_auto_sign
----+------+------------+------------------
 68 | ...  | t          | t
```

### 3. Mở terminal monitor logs
```bash
cd /home/giap/giap/Docuseal_Pro/test_certificates
./monitor_auto_sign.sh
```

---

## 🎯 BƯỚC TEST CHÍNH

### **Bước 1: Đăng nhập vào hệ thống**

1. Mở browser: **http://localhost:8081**
2. Login với tài khoản admin/user

---

### **Bước 2: Tạo hoặc mở Template**

**Option A - Tạo template mới:**
1. Click **"New Template"** hoặc **"Create Template"**
2. Upload PDF mẫu (bất kỳ PDF nào)
3. Thêm fields (Text, Signature, Date, etc.)
4. Click **"Save Template"**

**Option B - Dùng template có sẵn:**
1. Vào **"Templates"** list
2. Click vào template bất kỳ

---

### **Bước 3: Send Form đến Submitter**

1. Trong template, click **"Send"** hoặc **"New Submission"**

2. Điền thông tin submitter:
   - **Email:** (có thể dùng email của chính bạn)
   - **Name:** Test User
   - **Role:** (nếu có nhiều roles)

3. Click **"Send"** hoặc **"Submit"**

4. **Lưu ý submission ID** (ví dụ: #31, #32)

---

### **Bước 4: Điền và Submit Form**

**Option A - Nếu send cho chính mình:**
1. Check email → Click link trong email
2. Hoặc copy link từ UI

**Option B - Simulate submitter:**
1. Copy submission link
2. Mở incognito window
3. Paste link và mở

**Điền form:**
1. Điền tất cả required fields
2. Ký (nếu có signature field)
3. Click **"Submit"** ở cuối form

---

### **Bước 5: CHECK LOGS - Quan trọng nhất! 👀**

Ngay sau khi click Submit, quay lại terminal đang chạy `monitor_auto_sign.sh`

**Logs mong đợi (sau 1-2 giây):**

```
🔄 Auto-sign: Checking submission 31 for auto-sign eligibility
✅ Auto-sign: All submitters completed. Generating PDF...
📄 Auto-sign: PDF generated (2168931 bytes). Attempting to sign...
🔐 Auto-signing PDF for user 1...
✅ Found default certificate ID: 68
✍️  Signing with reason: 'Automatically signed upon completion'
✅ Auto-sign successful! Signed PDF size: 2172291 bytes
✅ Auto-sign: PDF signed successfully (2172291 bytes)
ℹ️  Auto-sign: Signed PDF ready. Storage integration pending.
```

**Nếu thấy logs trên → ✅ AUTO-SIGN THÀNH CÔNG!**

---

### **Bước 6: Download và Verify PDF**

#### **6A. Download PDF**

**Option 1 - Từ email:**
- Check email → Click "Download PDF"

**Option 2 - Từ UI:**
- Vào Submissions list
- Click vào submission vừa tạo
- Click "Download PDF"

**Option 3 - Từ database:**
```bash
# Check submission có PDF storage path
PGPASSWORD=letmesignpassword psql -h 192.168.90.11 -U letmesign -d letmesigndb \
  -c "SELECT id, template_id, status FROM submitters WHERE id = 31;"
```

#### **6B. Verify chữ ký bằng Adobe Reader**

1. **Mở PDF** bằng Adobe Acrobat Reader

2. **Xem Signatures Panel:**
   - Menu: `View` → `Show/Hide` → `Navigation Panes` → `Signatures`
   - Hoặc nhấn `Ctrl+Shift+F6`

3. **Kiểm tra thông tin:**
   ```
   📝 Signatures Panel:
      └─ CertificateSignature
         ├─ Signed by: AutoSign Fresh 1764821628
         ├─ Reason: Automatically signed upon completion
         ├─ Location: DocuSeal Pro Platform
         ├─ Date: December 4, 2025, XX:XX:XX
         └─ Status: ⚠️ UNKNOWN (self-signed cert)
   ```

4. **Click vào signature** → Xem chi tiết certificate

**✅ Thấy thông tin trên → PDF ĐÃ ĐƯỢC KÝ!**

#### **6C. Verify bằng command line**

```bash
# Install pdfsig nếu chưa có
sudo apt install poppler-utils

# Verify signature
pdfsig ~/Downloads/document.pdf
```

**Output mong đợi:**
```
Digital Signature Info of: document.pdf
Signature #1:
  - Signer Certificate Common Name: AutoSign Fresh 1764821628
  - Signing Time: Dec 04 11:23:45 2025 GMT
  - Signature Type: adbe.pkcs7.detached
  - Signature Validation: Signature is Valid.
```

#### **6D. Verify qua API**

```bash
# Lấy JWT token
TOKEN=$(curl -s -X POST "http://localhost:8081/api/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"your@email.com","password":"your_password"}' \
  | jq -r '.data.token')

# Verify PDF
curl -X POST "http://localhost:8081/api/pdf-signature/verify" \
  -H "Authorization: Bearer $TOKEN" \
  -F "pdf=@/path/to/document.pdf"
```

**Response mong đợi:**
```json
{
  "success": true,
  "data": {
    "is_signed": true,
    "signatures": [
      {
        "signer": "AutoSign Fresh 1764821628",
        "reason": "Automatically signed upon completion",
        "location": "DocuSeal Pro Platform",
        "is_valid": true
      }
    ]
  }
}
```

---

## 🔍 TROUBLESHOOTING

### ❌ Không thấy auto-sign logs?

**Check 1: Template có đúng user không?**
```bash
PGPASSWORD=letmesignpassword psql -h 192.168.90.11 -U letmesign -d letmesigndb \
  -c "SELECT id, name, user_id FROM templates WHERE id = <template_id>;"
```

**Check 2: Certificate thuộc đúng user không?**
```bash
PGPASSWORD=letmesignpassword psql -h 192.168.90.11 -U letmesign -d letmesigndb \
  -c "SELECT id, name, user_id, is_default, enable_auto_sign FROM certificates WHERE id = 68;"
```

**Check 3: Có error trong logs?**
```bash
grep -i "error\|failed" /tmp/docuseal.log | tail -20
```

---

### ⚠️ Logs có "Auto-sign failed"?

**Xem lỗi cụ thể:**
```bash
grep "Auto-sign failed" /tmp/docuseal.log | tail -5
```

**Lỗi thường gặp:**

1. **"Auto-sign password not set"**
   - Fix: Set lại password trong database
   ```bash
   PGPASSWORD=letmesignpassword psql -h 192.168.90.11 -U letmesign -d letmesigndb \
     -c "UPDATE certificates SET auto_sign_password_aes = 'test123456' WHERE id = 68;"
   ```

2. **"Default certificate not found"**
   - Fix: Set is_default = true
   ```bash
   PGPASSWORD=letmesignpassword psql -h 192.168.90.11 -U letmesign -d letmesigndb \
     -c "UPDATE certificates SET is_default = true, enable_auto_sign = true WHERE id = 68;"
   ```

3. **"Failed to generate PDF"**
   - Check submission data có đầy đủ không
   - Check template có fields hợp lệ không

---

### ⚠️ PDF không có chữ ký khi mở?

**Kiểm tra:**

1. **Có mở đúng PDF viewer không?**
   - ✅ Adobe Acrobat Reader → Có signatures panel
   - ❌ Chrome/Firefox → KHÔNG hiển thị signatures

2. **Có check signatures panel chưa?**
   - Menu: View → Navigation Panes → Signatures

3. **Verify bằng pdfsig:**
   ```bash
   pdfsig document.pdf
   # Nếu output "No signatures found" → PDF chưa được ký
   ```

---

## ✅ CHECKLIST TEST THÀNH CÔNG

- [ ] Server đang chạy (port 8081)
- [ ] Certificate 68 có is_default=true, enable_auto_sign=true
- [ ] Monitor logs đang chạy
- [ ] Đã send form đến submitter
- [ ] Đã điền và submit form
- [ ] **Thấy logs auto-sign trong terminal** ← Quan trọng nhất!
- [ ] Download được PDF
- [ ] Adobe Reader hiển thị signature trong panel
- [ ] pdfsig confirm "Signature is Valid"
- [ ] API verify trả về is_signed=true

**Nếu tất cả ✅ → AUTO-SIGN HOẠT ĐỘNG HOÀN HẢO!** 🎉

---

## 📊 SUMMARY

### Test thành công khi:

1. **Logs hiển thị:** `✅ Auto-sign successful! Signed PDF size: X bytes`
2. **Adobe Reader:** Signatures panel có "CertificateSignature"
3. **pdfsig:** Output "Signature is Valid"
4. **API:** Response `"is_signed": true`

### Không cần:

- ❌ Click button "Sign" sau submit
- ❌ Manual upload certificate
- ❌ Visible signature box trên PDF
- ❌ Chrome/Firefox hiển thị signature (chỉ Adobe Reader)

---

**Tạo bởi:** GitHub Copilot  
**Ngày:** 2025-12-04  
**Mục đích:** Test auto-sign bằng tay từng bước
