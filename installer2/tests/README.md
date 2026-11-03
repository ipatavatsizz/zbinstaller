# Test planı

Testler gerçek Steam/oyun kurulumuna yazmaz ve Steam'i kapatmaz. Kaynak fixture'lar salt okunur kullanılır; bütün yazmalar geçici dizinlerde yapılır.

## Rust çekirdek testleri

`cargo test -p installer-core` aşağıdakileri doğrular:

- JSON/BAT/shell/VDF yamaları, idempotency, diğer uygulama/ayarların korunması.
- Shebang, satır sonları, mevcut JVM seçenekleri ve eski Unix enjeksiyonunun kaldırılması.
- BOM, quoted/unquoted VDF, escape ve yorumlar; belirsiz hedeflerin reddedilmesi.
- Workshop kaydı ve hazır payload'ın aynı kütüphaneden seçilmesi.
- İndirme beklemesinin iptali ve zaman aşımı; hiçbir süreç işlemi çağrılmaz.
- Yazma hatası sonrası eski dosyaların geri yüklenmesi, yeni dosya/yedeklerin kaldırılması.
- Yarım kalmış journal'ın sonraki çalıştırmada kurtarılması ve izin verilmeyen hedeflerin reddi.
- İkinci dosya tanıtıcısı ve ayrı test sürecinin aynı kurulum kilidini alamaması.

## Fixture entegrasyonu

[Windows fixture testleri](tests/integration/installation_fixtures.rs) ve [platform eşlik testleri](tests/integration/platform_parity.rs), `cargo test -p installer2-tests` ile çalışır:

- Üç platform × iki branch için kurulum, yeniden kurulum ve kaldırma.
- Platforma uygun payload/flags, iç içe JSON yolu, `.sh`/`.command`/`start-server.sh` seçimi.
- Native Unix çalıştırma izninin korunması (Unix runner'larında).
- Eski Go yedeklerinin korunması ve ilk yedeğin yeniden kurulumda değişmemesi.
- Kaldırmadan önce yapılan kullanıcı ayarlarının korunması; nested Windows VM argümanlarının temizlenmesi.
- İkinci Steam kullanıcı girdisi bozuksa ilk kullanıcıya veya yedeğe hiçbir yazma yapılmaması.
- Standart/Flatpak/Snap/macOS yol adayları; oyun yokken kısmi diagnostics raporu.

Native adaptörler testlerde gerçek Steam'e yönlendirilmez. [Tauri log testi](../src-tauri/src/session_log.rs) yeniden açılışta log okuma ve diskten temizlemeyi doğrular.

## UI sözleşmeleri

`npm run test:ui`, Tauri API'sini mock'layarak branch seçimi, Workshop, indirme beklemesi/iptali, iki branch'in birlikte kilitlenmesi, ilerleme yüzdesi, bitiş/hata mesajları, F5/Escape, menü/kaldırma, diagnostics ve log temizleme davranışlarını sınar. DOM testlerinin gerçek native pencere davranışını doğruladığı iddia edilmez.

## Tam kontrol

```powershell
npm run build
npm run test:ui
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
npm run tauri build
```

[Üç platform workflow](../../.github/workflows/installer2.yml) aynı kontrolleri native runner'larda çalıştırır. Native UI/Steam/oyun kabulü ve imza gereksinimleri [migration checklist'te](../docs/MIGRATION.md) ayrı tutulur. Windows'ta üç hedefin dosya düzenini test etmek Linux/macOS native testi sayılmaz.
