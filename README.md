# Nova Client 0.3 — lightweight Minecraft Java launcher

Nova Client is an independent Tauri 2 desktop launcher prototype with a JavaScript/Vite interface and Rust game-launching backend.

## Included in this source snapshot

- Microsoft device-code sign-in through the CrustCore authenticator; the app does not request or store a Microsoft password.
- Minecraft Java release metadata and scanning of existing local version folders.
- Vanilla, Fabric, Forge, NeoForge, Quilt and Legacy Fabric launch profiles.
- A Fabric-focused optional performance mod preset.
- Manual JAR mod management and Modrinth search/install for compatible loaders.
- Server status lookup through mcsrvstat.us.
- Skin upload and equip/hide controls for cosmetics already owned by the signed-in Minecraft account.
- Low-memory launch presets (512 MB minimum; 1.5 GB default maximum), three concurrent downloads and a 1280×720 game window.

## Important limitations

This is **source code, not a tested release installer**. It has not yet passed a successful native build or end-to-end Minecraft launch test. Microsoft sign-in, loader installation, profile APIs and game launching need real-device verification.

The account session is held in process memory only and is not persisted securely between app restarts. Modrinth dependency chains are not resolved as complete modpacks. Existing game-root reuse is best-effort and does not import every third-party launcher's metadata. FPS depends on the PC, drivers, Java runtime, Minecraft version, render distance and mods; Nova cannot guarantee a target FPS.

Only legitimate Microsoft authentication and Java Edition ownership are supported. No cracked/offline mode or ownership bypass is included.

## Build on Arch Linux

Install prerequisites:

```bash
sudo pacman -Syu
sudo pacman -S --needed base-devel curl wget file openssl webkit2gtk-4.1 gtk3 librsvg libayatana-appindicator rustup nodejs npm
rustup default stable
```

Clone and build:

```bash
git clone https://github.com/sahilsingh549736-ui/nova-client.git
cd nova-client
npm install
npm run tauri dev
```

Create a Linux package:

```bash
npm run tauri build
```

Tauri build output is placed under `src-tauri/target/release/bundle/`.

## Windows and macOS

Build natively on each operating system with Node.js 22+, Rust stable, and the official Tauri prerequisites. The GitHub Actions workflow attempts Linux, Windows and macOS builds on pushes to `main`; artifacts are published only when each build job succeeds.

## Security and attribution

Never type a Microsoft password into Nova. Authentication should occur on Microsoft's device-login website. Review third-party mods before installing them. Nova is not affiliated with Mojang Studios, Microsoft, Modrinth or mcsrvstat.us.
