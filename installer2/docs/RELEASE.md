# Uygulama dağıtımı ve doğrulama

## Dağıtım ve yayın politikası

Araç kendisini sisteme kurmaz: Windows x64 EXE, Linux x64 çalıştırılabilir dosyası ve Apple Silicon macOS APP içeren ZIP indirilip doğrudan açılır. Setup kurucusu, sistem paketi veya DMG dağıtılmaz. Windows WebView2, Linux WebKitGTK 4.1 ve masaüstü bağımlılıkları gerektirir. Linux/macOS desteği deneyseldir; Intel macOS bu çıktılara dahil değildir.

Deneysel uygulamalar imzasız dağıtılır; Windows Authenticode ve macOS notarization uygulanmaz. SmartScreen/Gatekeeper uyarıları görülebilir. Native CI doğrulaması, bu son kullanıcı uyarı akışlarının veya tüm masaüstü dağıtımlarının testi değildir.

Installer kabulü [test ölçütlerine](../tests/README.md#installer-kabul-ölçütleri) dayanır: doğru platform payload ve başlatma seçenekleri, idempotent tekrar kurulum, ilgisiz ayarları koruyan kaldırma, ilk yedekler ve rollback/kurtarma. Değişiklikler oyun kapalıyken yapılır. Project Zomboid'i veya modu oyun içinde başlatmak installer kabulü ya da public yayın için önkoşul değildir. Testler geliştiricinin gerçek oyun dosyalarına yazmaz.

Yeni sürüm/adayı hazırlarken aynı kaynak commit'inde frontend build/UI/yayın-link testleri, Rust fmt/workspace testleri/Clippy ve üç native derleme doğrulanır. Linux/macOS'ta indirilen production dosyasının görünür ve boyanmış pencere smoke'u ile ayrı WebDriver feature build'indeki etkileşim testleri kaydedilir. Dosya adları ve indirilen dosyaların SHA-256 değerleri kaynak/run kimliğiyle birlikte tutulur. Test derlemesi ile production dosyası kanıtı birbirine aktarılmaz; her yeni aday kendi kanıtını gerektirir.

[Native workflow](../../.github/workflows/installer2.yml) üç uygulamayı toplar; isteğe bağlı taslak prerelease oluşturabilir. [Etiket workflow'u](../../.github/workflows/release.yml) aynı Rust doğrulamasını kullanır ve üç dosya ile `SHA256SUMS` içeren taslak hazırlar. Taslak hazırlama public yayın değildir; public yayın ayrı bir karardır. Bu kayıttaki doğrulama sırasında yeni tag/taslak/public release oluşturulmadı; aşağıdaki run'ın draft-release adımı atlandı.

## Son doğrulanan kaynak ve dosyalar

Kaynak commit'i **`6b3de94656fdc5f9b5ebb0e32ad72ee586f0d96e`**, [Actions run `42063139943`](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42063139943), 2026-11-05, sonuç `success`. Linux aynı kaynakla attempt 2'de yeniden doğrulandı; Windows/macOS aynı run'da başarılıdır. Build/UI/yayın-link/Rust test/fmt/Clippy şartları atlanmadı. Üç indirilen dosyanın hash'leri ve native kanıtlar bağımsız olarak kontrol edildi. Daha sonraki yalnız belge commit'leri bu test edilen kaynak kimliğini veya dosyaları değiştirmez.

| Hedef / native iş | İndirilebilir dosya | SHA-256 |
| --- | --- | --- |
| [Windows x64](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42063139943/job/127372724425) | `ZombieBuddyInstaller-windows-amd64.exe` | `19b33464ef9991c2f42f55a476bd48ff137376423815c1a51cfd8291233e0dbe` |
| [Linux x64](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42063139943/job/127372722742) | `ZombieBuddyInstaller-linux-amd64` | `aaa1712c34a1fe5600a43ea57f44458350d47f60209277f400fce520f00f44e2` |
| [macOS arm64](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42063139943/job/127372724895) | `ZombieBuddyInstaller-macos-arm64.zip` | `185261879050c85331995f30d174be44442249603d15cd764203800718a6ce38` |

Linux/macOS production dosyaları upload → download → `cmp` sonrasında içerikleri değiştirilmeden açıldı. [Linux smoke kanıtı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42063139943/artifacts/13432674646) ve [macOS smoke kanıtı](https://github.com/ipatavatsizz/zbinstaller/actions/runs/42063139943/artifacts/13432294759), `result.json`, `application.log`, `production-window.png` ile kaynak/hash/PID/görünür pencere/çizim sonucunu içerir. Son ekran görüntüleri ayrıca görsel incelendi. Aynı run'daki `installer2-native-ui-Linux` ve `installer2-native-ui-macOS`, [ayrı WDIO etkileşim kanıtıdır](../tests/native/unix-ui-acceptance.md). Production çizim kontrolü renk/parlaklık sezgisidir; tam etkileşim testi değildir.

Windows'un bu kaynak için sonucu CI/build/test/hash doğrulamasıdır; tarihsel bir EXE'nin native UI sonucu bu yeni binary'ye aktarılmaz. Bağımsız disk yedekli gerçek Steam config roundtrip ayrıca doğrulandı: hedefli gerçek patch, ilk backup/idempotency, ilgisiz seçeneklerin korunması, orijinal config/backup içerik-varlık-izin geri yüklemesi ve normal restart geçti. Canlı test `aed9b22` üzerindeki çalışma ağacında çalıştı ve test desteği `460b969` ile commit edildi. Son DACL karşılaştırması yalnız bilgilendirici `AI` bitini dışlar; `AR` dahil diğer kontrol bayrakları ve sıralı binary ACE'ler korunur. Son karşılaştırma izole regresyon/hosted Windows testleriyle doğrulandı; canlı Steam yazmaları tekrarlanmadı. Yerel özel kurtarma yolları `target/acceptance/windows-steam-config-step2-final.log` içinde kalır; config/yedek içerikleri yayın artifact'ı değildir.

Gerçek oyun kurulumu/oyun içi mod çalışması, gerçek Workshop indirme/abonelik matrisi ve manuel Gatekeeper/SmartScreen dağıtım akışı bu kanıtla doğrulanmış sayılmaz ve çalıştırılmadı. Bunlar oyun açılışını installer kabulüne veya public yayına önkoşul yapmaz. İzole fixture testleri ile native pencere testlerinin kapsamı [test belgesinde](../tests/README.md) ayrıdır. Linux CI'da mevcut 100 ms fixture indirme bütçesi aralıklı zaman aşımı üretebilir; yukarıdaki aynı kaynak yeniden denemesi bütün kontrolleri geçti.

## Tarihsel taslak

`v26.11.3-rust-candidate.20261104.2`, kaynak `c7662e9e9489a669381ed9315b2f39f0f2fe5612`: [önceki workflow](https://github.com/ipatavatsizz/zbinstaller/actions/runs/41903125865) ve [taslak prerelease](https://github.com/ipatavatsizz/zbinstaller/releases/tag/untagged-75932294f8141d3d5a54). Bu taslak yukarıdaki yeni kaynak/dosyalarla güncellenmedi; güncel artifact kimliği olarak kullanılmaz. Public yayın yapılmadı.
