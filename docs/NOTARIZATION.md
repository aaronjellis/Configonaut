# Notarization (macOS)

Produces a signed, hardened, notarized `.app` and `.dmg` that Gatekeeper
will accept on any Mac without "unidentified developer" warnings.

The Tauri-project side of notarization is already wired up:

- `src-tauri/tauri.conf.json` declares `hardenedRuntime: true` and
  points at `entitlements.plist` under the `bundle.macOS` block.
- `src-tauri/entitlements.plist` grants the minimum set of hardened
  runtime exceptions Tauri actually needs (JIT, unsigned executable
  memory, library validation off for bundled dylibs, network client).
  See the comments inside that file for why each exception is there.

What's deliberately *not* in the config is the signing identity and
the notarization credentials — those come from environment variables
at build time so no secrets or personal identity strings live in the
repo.

## One-time account setup

You only need to do this once, and if you set it up yesterday for any
other signed Mac project (including the Swift build of Configonaut),
it still works — the cert, password, and team ID are account-level.

1. Active Apple Developer membership ($99/yr).
2. A **Developer ID Application** certificate in your login Keychain.
   Xcode → Settings → Accounts → team → Manage Certificates → `+`.
3. An **app-specific password** for notarytool, generated at
   appleid.apple.com → Sign-In and Security → App-Specific Passwords.
4. Your **Team ID** (10-character string, visible on
   developer.apple.com/account → Membership).

## Environment variables

Tauri's build reads these four at build time. Set them in your shell
before running a release build:

```bash
export APPLE_ID="you@example.com"
export APPLE_PASSWORD="xxxx-xxxx-xxxx-xxxx"   # app-specific password
export APPLE_TEAM_ID="ABCD123456"             # your 10-char Team ID
export APPLE_SIGNING_IDENTITY="Developer ID Application: aaronjellis (ABCD123456)"
```

The `APPLE_SIGNING_IDENTITY` string has to match the full Common Name
of your Developer ID cert exactly. Find it with:

```bash
security find-identity -v -p codesigning
```

Look for the `Developer ID Application: ...` line and copy everything
after the hex SHA.

These can live in a local `.envrc` (direnv), a 1Password shell plugin,
`~/.zshenv`, or a gitignored `.env.notarize` you `source` before
building. **Do not commit them.** If you use a flat file, add it to
`.gitignore`.

## Build + notarize

Once the env vars are set:

```bash
cd tauri-app
bun run tauri build
```

With all four env vars present, Tauri will:

1. Compile and bundle the app as usual.
2. Sign `Configonaut.app` with your Developer ID cert, applying the
   hardened runtime and the entitlements in `entitlements.plist`.
3. Zip the signed app and submit to Apple's notary service via
   `xcrun notarytool submit --wait`.
4. On approval, staple the notarization ticket to the app with
   `xcrun stapler staple`.
5. Package the final `.dmg` around the stapled app.

Apple's notary queue usually takes 30 seconds to a few minutes. The
build blocks on it, so don't kill the process if it looks stalled
during the submit step.

## Verifying the result

After a successful build, check the output at
`src-tauri/target/release/bundle/macos/Configonaut.app`:

```bash
# Signature + entitlements
codesign -dv --verbose=4 src-tauri/target/release/bundle/macos/Configonaut.app

# Confirm the notarization ticket is stapled
xcrun stapler validate src-tauri/target/release/bundle/macos/Configonaut.app

# Confirm Gatekeeper will accept it on a fresh machine
spctl -a -vvv -t install src-tauri/target/release/bundle/macos/Configonaut.app
```

The last command should print `accepted` and
`source=Notarized Developer ID`. If you see `rejected` or
`source=Unnotarized Developer ID`, something in the notarize step
failed silently — grab the submission UUID from the Tauri build log
and run:

```bash
xcrun notarytool log <uuid> \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_PASSWORD"
```

to see Apple's structured rejection reasons. The usual culprits are:

- A bundled dylib missing hardened runtime → add signing flags or
  revisit `disable-library-validation`.
- A missing entitlement the WebView actually needs → add it to
  `entitlements.plist` and rebuild.
- Wrong `minimumSystemVersion` vs what the binary was linked against.

## Unsigned dev builds still work

`bun run tauri dev` produces a debug build that isn't signed or
notarized — the env vars are only consulted by `tauri build`. You can
iterate on the app day-to-day without any of this in place. This doc
only matters when you're cutting a release you want to hand to another
person.
