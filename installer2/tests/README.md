# Test planı

Bu proje Windows üzerinde Steam ve Project Zomboid dosyalarıyla çalışan bir Tauri uygulamasıdır. Testler üç katmanda ayrılır; örnek girdiler `data/` altındadır ve hiçbir test gerçek Steam kurulumuna yazmaz.

## 1. Rust birim testleri

`tests/unit/patch_fixtures.rs`, saf dönüştürme ve doğrulama işlevlerini test eder:

- JSON, BAT ve VDF patch'lerinin beklenen değişikliği yapması ve diğer ayarları koruması.
- Tekrar çalıştırıldığında aynı sonucu vermesi (idempotency) ve argümanları çoğaltmaması.
- Branch kimliklerinin serileştirilmesi.
- `appworkshop_108600.acf` içindeki `WorkshopItemsInstalled` kayıtlarının LTS/Beta Workshop ID’leriyle eşleşmesi.
- Bozuk JSON/VDF girdilerinin yazılabilir çıktı üretmeden reddedilmesi.

## 2. Dosya sistemi entegrasyon testleri

`tests/integration/installation_fixtures.rs`, gerçek dosya sistemi davranışını geçici dizinlerde doğrular:

- Açıkça verilen Steam/game/config yollarından keşif ve ikincil Steam library çözümleme.
- İndirme takibinde Steam manifestinin ve payload dosyalarının birlikte hazır olmasının doğrulanması.
- Fixture'ların patch edilmesi, DLL/JAR payload'larının kopyalanması ve tekrar çalıştırmada idempotency.
- Patch doğrulaması başarısızsa hiçbir kurulum dosyasının değiştirilmemesi.
- Uninstall sırasında yalnızca ZombieBuddy payload'larının ve Project Zomboid'e ait launch işaretlerinin kaldırılması; diğer Steam seçeneklerinin korunması.
- Diagnostics patch aksiyonlarının ilk yedek davranışı ve içerik koruması.

Geçici dizinler her test sonunda kaldırılır; fixture'lar kaynak olarak salt okunur kullanılır.

## 3. UI sözleşme testleri

`ui/` altındaki Vitest/jsdom testleri Tauri komutlarını mock'layarak frontend-backend sınırını test eder:

- Branch sürümlerinin gösterilmesi; indirilmemiş branch için Workshop açılması ve hazır branch kurulumu.
- F5 ile diagnostics penceresi açma ve sağ tıkla Workshop açma.
- Kurulum sürerken butonların kilitlenmesi ve ilerleme mesajının gösterilmesi.
- Diagnostics raporunun gösterilmesi, her patch aksiyonunun doğru komuta yönlenmesi ve log temizleme.

## Çalıştırma

`installer2/` dizininden:

```powershell
cargo test -p installer2-tests
npm run test:ui
npm run build
cargo check -p installer2
```

Gerçek Windows Registry, Steam süreci, WebView pencere odağı ve Steam istemcisinin başlatılması host entegrasyonlarıdır; bunları CI'da değiştirmek yerine Windows test makinesinde kontrollü kabul testi olarak doğrulayın.
