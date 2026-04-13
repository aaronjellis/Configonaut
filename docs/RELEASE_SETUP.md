# Release Setup

One-time setup for automated builds and auto-updates via GitHub Actions.

## 1. Generate the Tauri updater keypair

This keypair is how running instances of Configonaut verify that an update
is legitimate (separate from Apple codesign — this is Tauri's own mechanism).

```bash
cd tauri-app
bun run tauri signer generate -w ~/.tauri/configonaut.key
```

This creates two files:
- `~/.tauri/configonaut.key` — **private key** (never commit this)
- `~/.tauri/configonaut.key.pub` — **public key**

Copy the public key contents and replace `UPDATER_PUBKEY_PLACEHOLDER` in
`src-tauri/tauri.conf.json` → `plugins.updater.pubkey`.

## 2. Export your Apple Developer ID certificate as .p12

Open Keychain Access → find "Developer ID Application: YOUR_NAME (YOUR_TEAM_ID)"
→ right-click → Export → save as `.p12` with a password.

Then base64-encode it:

```bash
base64 -i ~/Desktop/DeveloperID.p12 | pbcopy
```

## 3. Add GitHub repository secrets

Go to your repo → Settings → Secrets and variables → Actions → New repository secret.

| Secret name                          | Value                                              |
| ------------------------------------ | -------------------------------------------------- |
| `TAURI_SIGNING_PRIVATE_KEY`          | Contents of `~/.tauri/configonaut.key`              |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Passphrase you chose (or empty string)              |
| `APPLE_CERTIFICATE`                  | Base64 of the .p12 (from step 2)                    |
| `APPLE_CERTIFICATE_PASSWORD`         | Password you set when exporting the .p12            |
| `APPLE_ID`                           | Your Apple ID email                                 |
| `APPLE_PASSWORD`                     | App-specific password (the one you stored earlier)  |
| `APPLE_TEAM_ID`                      | `ABCD123456`                                        |

## 4. Replace the updater public key placeholder

Edit `tauri-app/src-tauri/tauri.conf.json` and replace:

```json
"pubkey": "UPDATER_PUBKEY_PLACEHOLDER"
```

with the actual contents of `~/.tauri/configonaut.key.pub`.

## 5. Release

```bash
# Bump version in tauri-app/src-tauri/tauri.conf.json and tauri-app/package.json
# Commit, tag, push:
git tag v1.0.0
git push origin v1.0.0
```

GitHub Actions will:
1. Build macOS (universal binary: ARM + Intel) and Windows (x64)
2. Sign and notarize the macOS build
3. Generate updater `.sig` files
4. Create a **draft** GitHub Release with all artifacts + `latest.json`

Review the draft release on GitHub and click "Publish" when ready. Running
instances of Configonaut will detect the update on next launch via `latest.json`.

## How auto-update works

On launch, the app checks:
```
https://github.com/aaronjellis/Configonaut/releases/latest/download/latest.json
```

If the version in `latest.json` is newer than the running version, a dialog
prompts the user to update. The download, signature verification, and
install happen automatically. The user just restarts the app.
