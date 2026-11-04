# Rust uygulama adayları

## Deneysel dağıtım ve imza politikası

2026-11-04: mevcut Go yayın akışındaki imzasız dağıtım, Rust deneysel adaylarında da korunur. Windows EXE Authenticode ile imzalanmaz. Linux dosyası ve macOS APP ZIP'i imzasızdır; macOS notarization uygulanmaz. Sertifika veya imzalama secret'ı bu çalışma için gerekli değildir. Windows SmartScreen ve macOS Gatekeeper uyarıları görülebilir. Bu kayıt imzalı veya notarized dağıtım kabulü değildir.

Aracın kendisi kurulmaz: Windows EXE, Linux çalıştırılabilir dosyası, macOS APP içeren ZIP indirilir ve doğrudan açılır. Windows WebView2, Linux WebKitGTK 4.1 gerektirir. Linux/macOS desteği deneysel olarak etiketlenir. Native CI sonuçları gerçek Steam/oyun veya tüm masaüstü dağıtımlarının kabulü değildir.

## Adayın doğrulanması

1. Aynı commit'te frontend build/UI testleri, Rust fmt/test/Clippy ve üç platform derlemesi başarılı olmalı.
2. Windows izole süreç ve başlatıcı kabul testleri workspace testlerine dahildir. Linux/macOS test derlemesi gerçek Tauri/WebKit pencerelerinde native UI suite'ini geçmeli.
3. İndirilebilir release dosyaları WebDriver feature/config olmadan derlenir. Windows'ta indirilen EXE native UI testinden geçirilir.
4. Taslakta yalnızca üç uygulama dosyası ile SHA-256 doğrulama listesi bulunur; UI ekran görüntüleri ayrı Actions artifact'larıdır.
5. Aday commit'i, workflow bağlantısı, dosya adları ve Windows sonucu [geçiş kaydına](MIGRATION.md) yazılır. Başarısız testte aday kabul edilmez.

Yeni adaylar önce taslak prerelease olarak hazırlanır. Taslağın public yayımlanması ayrı yayın adımıdır. Eski Go kaynakları referans/geri dönüş için korunur; Rust uygulamasını derlemek veya çalıştırmak Go gerektirmez. Linux/macOS manuel masaüstü ve gerçek Steam/oyun kabulleri, [kabul tablosunda](MIGRATION.md#kabul-sonuç-kaydı) açık kalır.
