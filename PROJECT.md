# PasLsp — Pascal Language Server

Proyek LSP (Language Server Protocol) untuk bahasa Pascal, dibangun dari awal dengan Rust + `tower-lsp`, target di Termux (aarch64), digunakan bersama NeoVim v0.12.6.

## Lokasi project

- Root project: `/data/data/com.termux/files/home/storage/projects/PasLsp`
- Binary hasil build: `/data/data/com.termux/files/home/storage/projects/PasLsp/target/release/paslsp`
- Konfigurasi NeoVim: `~/.config/nvim/init.lua`

## Struktur file

```
PasLsp/
├── Cargo.toml              # dependensi Rust (tower-lsp, tokio, serde, url)
├── Cargo.lock
├── src/
│   ├── main.rs             # titik masuk LSP server (stdin/stdout JSON-RPC)
│   ├── server.rs           # handler LSP: initialize, didOpen/Change/Close,
│   │                       #   completion, hover, goto_definition, references,
│   │                       #   document_symbol, diagnostics
│   └── parser.rs           # lexer + parser Pascal sederhana, symbol table (vars & procs)
└── target/release/paslsp   # binary siap pakai (build --release)
```

## Fitur yang sudah berfungsi

| Fitur | Status | Keterangan |
|-------|--------|------------|
| `initialize` / `shutdown` | ✅ | Handshake LSP standar |
| Text document sync (didOpen/didChange/didClose) | ✅ | Simpan isi file `.pas` di memori |
| Diagnostics | ⚠️ | Deteksi `begin`/`end` tidak sepasang, `program` tanpa titik koma. Masih **false positive** karena `end;` dan `end.` tidak dikenali (lihat Masalah Terkini di bawah) |
| Completion | ✅ | Kata kunci Pascal + variabel + prosedur/fungsi dari symbol table |
| Hover | ✅ | Info tipe variabel / deklarasi prosedur-fungsi |
| Go-to-definition | ✅ | Lompat ke deklarasi variabel / prosedur / fungsi |
| Find-references | ✅ | Daftar semua pemakai symbol (deklarasi + penggunaan) |
| Document symbols | ✅ | Outline: variabel + prosedur/fungsi |
| Parser AST + symbol table | ✅ | Lexer + parser manual, menangkap `program`, `var`, `procedure`, `function`, `begin`/`end` |

## Bagaimana build

Di dalam folder `PasLsp`:

```sh
cargo build --release
```

Binary dihasilkan di `target/release/paslsp`.

## Bagaimana pakai di NeoVim

Tambahkan ke `~/.config/nvim/init.lua`:

```lua
-- bootstrap lazy.nvim, LazyVim and your plugins
require("config.lazy")

vim.lsp.config('paslsp', {
  cmd = { "/data/data/com.termux/files/home/storage/projects/PasLsp/target/release/paslsp" },
  filetypes = { "pascal" },
  root_dir = vim.fn.getcwd(),
})
vim.lsp.enable('paslsp')
```

Buka file `.pas`:

```sh
nvim /path/to/file.pas
```

Keymap LSP bawaan NeoVim yang bisa dicoba:
- `gd` — go to definition
- `gr` — find references
- `K` — hover
- `<leader>ds` — document symbols (outline)
- Completion otomatis saat mengetik (tergantung nvim-cmp / built-in omnicompletion)

Verifikasi LSP aktif (di dalam NeoVim):

```lua
:lua print(vim.inspect(vim.lsp.get_clients({name='paslsp'})))
```

## File uji contoh

Contoh file Pascal yang sudah diuji (`test_full.pas`):

```pascal
program test;

var
    x, y: integer;
    msg: string;

procedure add(a, b: integer);
begin
    x := a + b;
end;

function double(n: integer): integer;
begin
    double := n * 2;
end;

begin
    x := 10;
    y := 20;
    add(x, y);
    msg := 'hello';
end.
```

## Masalah terkini (perlu diperbaiki sesi berikutnya)

### 1. False positive diagnostics `begin`/`end`

Di `src/server.rs`, fungsi `check_pascal` menggunakan exact string match:

```rust
if trimmed == "begin" { begin_count += 1; }
if trimmed == "end" { end_count += 1; }
```

Ini gagal menangkap:
- `end;` (titik koma di belakang)
- `end.` (titik di akhir program)
- `begin ` (diikuti statement di baris sama)

Contoh file `biodata.pas` di `~/storage/projects/Lab-Dasar-Pemrograman-3/pertemuan1/biodata.pas`:
- Baris 7: `  begin` → terdeteksi
- Baris 11: `begin` → terdeteksi
- Baris 9: `  end;` → **tidak** terdeteksi (karena `;`)
- Baris 19: `end.` → **tidak** terdeteksi (karena `.`)

Hasil: error palsu "unmatched begin/end: 2 begin(s), 0 end(s)".

**Perbaikan yang dibutuhkan:** Ganti dengan pemeriksaan awalan kata:

```rust
fn is_begin_kw(trimmed: &str) -> bool {
    trimmed == "begin" || trimmed.starts_with("begin ")
}

fn is_end_kw(trimmed: &str) -> bool {
    trimmed == "end" ||
    trimmed.starts_with("end;") ||
    trimmed.starts_with("end.") ||
    trimmed.starts_with("end ")
}
```

Di loop `check_pascal`, ganti pemanggilan sesuai.

### 2. Referensi duplikat

Di fungsi `references`, deklarasi dan penggunaan sama-sama dipindai dari teks, sehingga deklarasi muncul 2 kali dalam hasil referensi. Perlu deduplikasi berdasarkan `(uri, line, character)`.

### 3. Completion trigger karakter

Hanya `.` yang diatur sebagai trigger. Bisa ditambahkan trigger untuk huruf atau spasi agar completion muncul saat mengetik identifier.

## Rencana sesi berikutnya (prioritas)

1. ✅ **Perbaiki `check_pascal`** agar `end;` dan `end.` terdeteksi (hapus false positive di `biodata.pas`) — selesai.
2. ✅ **Deduplikasi referensi** di `references` — selesai.
3. **Uji kembali** di NeoVim dengan file `biodata.pas` dan `test_full.pas`.
4. **Publish ke Termux package repository** (termasih opsi, fase terakhir): buat `build.sh`, daftar di `termux/packages`, ajukan PR.

## Catatan teknis

- Bahasa implementasi: Rust (edisi 2024)
- Library LSP: `tower-lsp` v0.19
- Async runtime: `tokio` dengan fitur `rt-multi-thread`, `io-std`, `macros`
- URL parsing: `url` v2
- Serialisasi: `serde`, `serde_json`
- Parser: ditulis manual (lexer + recursive descent sederhana), bukan crate parsing pihak ketiga
- Target platform: aarch64 Linux (Termux)
- Editor target: NeoVim v0.12.6

## Perintah cepat

```sh
# Build
cd /data/data/com.termux/files/home/storage/projects/PasLsp
cargo build --release

# Test parser (unit test)
cargo test --release

# Jalankan server secara manual (untuk debugging JSON-RPC)
printf 'Content-Length: %d\r\n\r\n%s' "$(echo -n 'PAYLOAD' | wc -c)" 'PAYLOAD' | ./target/release/paslsp

# Uji di NeoVim
nvim file.pas
```
