class L33t0Emojify < Formula
  desc "Generate platform-compatible custom emoji images"
  homepage "https://github.com/l33t0/emojify"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-aarch64-apple-darwin.tar.gz"
      sha256 "2163ada4a51a0471e4e72993b01c772930faf626afb8fe2b3e5832a6f78933e6"
    end
    on_intel do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-x86_64-apple-darwin.tar.gz"
      sha256 "48d0562ce2067b3b62e3f847fc8d1f3e34d7517d09390e29f2a63754806b8c17"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e6c61a83ff0ef74bc551dce00347da26cbcb628e5dfdc04f1b24e993302bb0ed"
    end
    on_intel do
      url "https://github.com/l33t0/emojify/releases/download/v#{version}/emojify-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0b73a8d05f3bd167b846c5a9afb506ebf3236595f546c0055e7f6217ec972132"
    end
  end

  def install
    bin.install "emojify"
  end

  test do
    system bin/"emojify", "--version"
  end
end
