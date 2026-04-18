# 🎚️ AltFade

An elegant, system-level **Audio Ducking** utility designed for gamers who love listening to their own music or podcasts while playing. 

Tired of audio chaos when you Alt-Tab to hit play on a YouTube video or Spotify playlist? AltFade listens to the Windows Core Audio APIs and automatically lowers your focused game's volume to 20% the moment background media starts playing. When you pause the music, your game volume flawlessly returns to its original level. 

Perfect for long, atmospheric exploration sessions in Skyrim or tactical operations in Arma 3, where you want to seamlessly blend your own soundtrack without completely muting the world.

## ✨ Why AltFade?
- **Zero Audio Clutter:** No more manual volume adjustments through the Windows Mixer every time you change a track.
- **Performance First:** Built with Tauri and Rust. It sits quietly in your system tray with a virtually non-existent memory and CPU footprint, saving all your system resources for your games.
- **Deep OS Integration:** Utilizes Windows SMTC and WASAPI to strictly manipulate the specific game's audio channel without affecting your master system volume.

## 🚀 Key Features
* **Smart Media Detection:** Real-time tracking of the global Windows media state (Playing/Paused/Stopped).
* **Targeted Muting:** Select your specific game process. AltFade only ducks the targeted application, leaving everything else untouched.
* **Modern & Lightweight UI:** A clean, accessible frontend that lives in your system tray for quick toggling and process selection.
