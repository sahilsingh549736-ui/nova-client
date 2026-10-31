# Nova Client

A lightweight, cross-platform Minecraft Java launcher built with Tauri 2, Rust, and Vite.

**Status:** early development. Native builds, Microsoft sign-in, and game launching must be tested before release.

## Development

Requires Node.js, Rust stable, and Tauri 2 platform dependencies.

```bash
npm install
npm run tauri dev
```

Build a release bundle:

```bash
npm run tauri build
```

## Authentication

Nova is intended to use Microsoft's official sign-in flow for accounts that own Minecraft: Java Edition. It does not bypass ownership checks or implement cracked/offline authentication.

## Planned features

- Lightweight desktop interface
- Vanilla and mod-loader profiles
- Import existing instances
- Mod and modpack management
- Multiplayer server list and status
- Skin and cape management
- Windows, Linux, and macOS packaging

## Release checklist

- [ ] Frontend build verified
- [ ] Linux native build verified
- [ ] Windows native build verified
- [ ] macOS native build verified
- [ ] Microsoft sign-in and ownership flow tested
- [ ] Vanilla and modded launch smoke tests passed
