# 🔊 AltFade

**Automatically ducks your game audio when media starts playing — and smoothly fades it back when you pause.**

---

## ✨ Features

- 🎮 **Per-process audio ducking** — select one or more running games/apps as targets
- 🎵 **SMTC integration** — detects YouTube, Spotify, and any Windows media source automatically
- 🎯 **Trigger app filtering** — optionally choose which media apps are allowed to trigger ducking
- 🔉 **Smooth fade transitions** — volume eases down when media plays, gently fades back up when paused
- ⏱️ **Restore delay** — holds the duck through track changes so volume doesn't pump between songs
- 🎚️ **Configurable duck level** — set your preferred volume percentage (5% – 50%); your original per-app volume is always restored
- 📊 **Live level meters** — see which session is actually making noise
- ⌨️ **Global hotkey** — toggle the service from inside a game with `Ctrl+Alt+D`
- 🎧 **All output devices** — finds sessions on every active playback device, not just the default one
- 🖼️ **Process icons** — see app icons next to each audio session
- 🗂️ **System tray** — runs silently in the background, closes to tray; tray icon shows live status
- 🚀 **Auto-start** — optionally launch minimized on Windows startup, resuming your saved setup
- 🌗 **Dark / Light mode** — follows your Windows theme by default
- 🌍 **Turkish & English** language support
- ⚡ **Lightweight** — built with Tauri v2 + Rust, minimal resource usage

## 📥 Installation

Download the latest `AltFade_x64-setup.exe` from the [Releases](../../releases) page and run the installer. No additional dependencies required.

## 📝 Notes

- Games running **as administrator** (many titles with anti-cheat) cannot be controlled unless AltFade is also run as administrator.
- All settings (targets, duck level, delay, language, service state) persist across restarts in `%APPDATA%\com.sercannaya.altfade\config.json`.

## 🛠️ Built With

Tauri v2 · Rust · Vue 3 · TypeScript · Windows Core Audio API (WASAPI) · SMTC

---

# 🔊 AltFade

**Oyun oynarken arka planda müzik veya video açıldığında oyunun sesini otomatik olarak kısar — medya durduğunda sesi yumuşakça geri getirir.**

---

## ✨ Özellikler

- 🎮 **Hedefe özel ses kısma** — bir veya birden fazla oyun/uygulamayı hedef olarak seç
- 🎵 **SMTC entegrasyonu** — YouTube, Spotify ve tüm Windows medya kaynaklarını otomatik algılar
- 🎯 **Tetikleyici filtresi** — istersen yalnızca seçtiğin medya uygulamaları kısmayı tetiklesin
- 🔉 **Yumuşak geçişler** — medya başlayınca ses yavaşça kısılır, durduğunda nazikçe geri gelir
- ⏱️ **Geri yükleme gecikmesi** — şarkı geçişlerinde ses inip kalkmaz, kısık kalır
- 🎚️ **Ayarlanabilir kısma seviyesi** — istediğin ses yüzdesini belirle (%5 – %50); uygulamanın orijinal ses seviyesi her zaman korunur
- 📊 **Canlı ses göstergeleri** — hangi oturumun gerçekten ses çıkardığını gör
- ⌨️ **Genel kısayol** — oyun içindeyken `Ctrl+Alt+D` ile servisi aç/kapat
- 🎧 **Tüm ses aygıtları** — yalnızca varsayılan değil, tüm aktif çıkış aygıtlarındaki oturumları bulur
- 🖼️ **Uygulama ikonları** — ses oturumlarının yanında program ikonları görünür
- 🗂️ **Sistem tepsisi** — arka planda sessizce çalışır; tray ikonu anlık durumu gösterir
- 🚀 **Otomatik başlangıç** — Windows açılışında küçültülmüş başlar, kayıtlı ayarlarınla devam eder
- 🌗 **Koyu / Açık tema** — varsayılan olarak Windows temasını takip eder
- 🌍 **Türkçe & İngilizce** dil desteği
- ⚡ **Hafif** — Tauri v2 + Rust ile inşa edildi, minimum kaynak kullanımı

## 📥 Kurulum

[Releases](../../releases) sayfasından en güncel `AltFade_x64-setup.exe` dosyasını indirip çalıştır. Ek bağımlılık gerekmez.

## 📝 Notlar

- **Yönetici olarak çalışan oyunlar** (anti-cheat kullanan birçok oyun) ancak AltFade de yönetici olarak çalıştırılırsa kontrol edilebilir.
- Tüm ayarlar (hedefler, kısma seviyesi, gecikme, dil, servis durumu) `%APPDATA%\com.sercannaya.altfade\config.json` içinde saklanır ve yeniden başlatmalarda korunur.

## 🛠️ Kullanılan Teknolojiler

Tauri v2 · Rust · Vue 3 · TypeScript · Windows Core Audio API (WASAPI) · SMTC
