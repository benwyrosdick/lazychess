# Homebrew Tap Setup Guide

This guide explains how to set up the Homebrew tap for lazychess.

## 1. Set Up the Homebrew Tap Repository

The tap repository at [benwyrosdick/homebrew-tap](https://github.com/benwyrosdick/homebrew-tap) should already exist (shared with restui). Add the lazychess formula to it:

```
homebrew-tap/
├── Formula/
│   ├── restui.rb
│   └── lazychess.rb          # Copy from this directory
└── .github/
    └── workflows/
        └── update-formula.yml  # Update with the version from this directory
```

Copy `lazychess.rb` into `Formula/` in the tap repository.

Update `.github/workflows/update-formula.yml` in the tap repository with the version from this directory. The updated workflow uses `github.event.client_payload.formula` to dynamically determine which formula to update, making it work for both restui and lazychess.

## 2. Ensure the Personal Access Token is Configured

The `HOMEBREW_TAP_TOKEN` secret should already be set up in your GitHub account for the restui repository. You need to ensure the same secret is also available in the **lazychess** repository:

1. Go to the lazychess repo: Settings > Secrets and variables > Actions
2. Click "New repository secret"
3. Name: `HOMEBREW_TAP_TOKEN`
4. Value: use the same PAT (or create a new one at https://github.com/settings/tokens with `repo` scope)

## 3. Create a Release

Create and push a version tag to trigger the first release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

This will:
1. Build binaries for macOS (Intel + Apple Silicon) and Linux
2. Create a GitHub release with the binaries
3. Trigger the homebrew-tap to update the formula with correct SHA256 checksums

## 4. Manual Formula Update (if needed)

If automatic updates fail, manually update the formula:

```bash
# Download the release and get SHA256
curl -L https://github.com/benwyrosdick/lazychess/releases/download/v0.1.0/lazychess-aarch64-apple-darwin.tar.gz -o arm.tar.gz
shasum -a 256 arm.tar.gz

curl -L https://github.com/benwyrosdick/lazychess/releases/download/v0.1.0/lazychess-x86_64-apple-darwin.tar.gz -o x86.tar.gz
shasum -a 256 x86.tar.gz

curl -L https://github.com/benwyrosdick/lazychess/releases/download/v0.1.0/lazychess-x86_64-unknown-linux-gnu.tar.gz -o linux.tar.gz
shasum -a 256 linux.tar.gz
```

Then update the SHA256 values in `Formula/lazychess.rb` in the tap repo.

## 5. Users Can Install With

```bash
brew tap benwyrosdick/tap
brew install lazychess
```

Or in one command:
```bash
brew install benwyrosdick/tap/lazychess
```

## Troubleshooting

### Build fails on Apple Silicon
Make sure the workflow uses `macos-latest` which now runs on ARM.

### Formula test fails
Ensure lazychess supports `--version` flag and outputs the version number (it does via clap).

### Automatic update doesn't trigger
Check that:
- `HOMEBREW_TAP_TOKEN` secret is set correctly in the lazychess repo
- Token has `repo` scope
- The tap repository exists at `benwyrosdick/homebrew-tap`

### Stockfish dependency
The formula includes `depends_on "stockfish" => :recommended` so Homebrew will suggest installing Stockfish but won't require it. Users can skip it with `brew install --without-stockfish benwyrosdick/tap/lazychess` if they have Stockfish installed another way.
