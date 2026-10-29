# Stage 10: Body form, cookie jar, variabel dinamis, dan Copy as code

Suite [`stage10-e2e.mjs`](../../../e2e/stage10-e2e.mjs) · 37 cek

Mengirim body form-data dengan file yang dipilih, URL-encoded, dan binary ke server lokal, lalu memeriksa apa yang benar-benar diterima server. Suite ini juga menguji variabel dinamis seperti `{{$guid}}`, duplikat request, section, dan collection, dialog Copy as code (dropdown Language dan Values, environment variable sebagai bawaan, bahasa yang diingat), export Collection v2.1, dan cookie jar yang tersimpan terenkripsi serta tetap ada setelah aplikasi dibuka ulang. Kode hasil Copy as code dijalankan sungguhan di [codegen](codegen.md).

## Kebutuhan

- Server HTTP lokal yang dibuat oleh suite itu sendiri (Node `http`).
- Dialog simpan atau buka file Windows, dijawab oleh `answer-dialog.ps1`.

## Alur

### 1. Local server

Menyalakan server lokal yang dipakai langkah berikutnya.

### 2. CDP

Terhubung ke salinan uji lewat CDP dan memasang helper.

### 3. Form data with a picked file

- form file is picked through the system dialog
- form data is sent as multipart with the file
- a file that was not picked cannot be uploaded
- form parts cannot point at unpicked files either
- form field names with line breaks are refused

### 4. URL-encoded shares the rows; file rows are left out

- URL-encoded mode explains that file rows are skipped
- URL-encoded body is sent

### 5. Binary file body

- binary file is streamed with its length
- form rows and the picked file are saved

### 6. Duplicate request

- Duplicate opens the copy right after the original
- copy sits right after the original

### 7. Dynamic variables

- preview keeps dynamic variables and does not flag them
- {{$guid}} and {{$randomInt}} are filled in when sending
- autocomplete offers dynamic variables
- unsupported dynamic variables are reported

### 8. Copy as code

- language and values are dropdowns, with eight languages and environment variables first
- cURL reads variables from the environment and uses --form for files
- fetch code builds a FormData and reads the file in Node
- Python code sends files with requests
- the language is remembered; values start on environment variables again
- "Filled in" resolves through the Rust core
- Copy puts the code on the clipboard

### 9. Cookie jar

- response Cookies tab shows the cookie it set
- the jar sends the cookie back
- a typed Cookie header wins over the jar
- Cookies view lists the cookie with its value hidden
- Show reveals the value

### 10. Duplicate section + collection, export

- Duplicate section copies it with its requests
- Export writes a Postman v2.1 collection
- export leaves out local folders and secret values
- the export imports back into Codestamp
- Duplicate collection copies requests and secret variables
- Globals export as a Postman globals file

### 11. The jar survives a restart (stored encrypted)

- cookies are still there after a restart
- the cookie value is not stored in plain text
- a cookie can be deleted
- test data cleaned up

---

[Daftar stage](../README.md#daftar-stage) · Berikutnya: [Collaboration lewat Git](stage-11.md)
