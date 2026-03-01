class Emojify < Formula
  desc "Generate platform-compatible custom emoji images"
  homepage "https://github.com/l33t0/emojify"
  version "0.1.0"
  license "MIT"

  # SHA256 checksums are populated automatically by the release workflow.
  # Update these values after each release using:
  #   curl -sL <url> | shasum -a 256
  on_macos do
    on_arm do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "emojify"
  end

  test do
    system bin/"emojify", "--version"
  end
end
